use crate::config::EasyPlaceConfig;
use crate::services::{GhostRay, PlaceNeeds};
use crate::structs::block::Face;
use crate::structs::easy_place::{
    Aim, AimKind, EasyPlaceNotice, EasyPlaceTelemetry, FaceRule, GameMode, Half, HoldState, InventorySlot,
    PendingPlace, PendingSet, PlaceNeed, Placement, RayLimits, SpotWorld, SwapAnswer, TriedSet, UseCommand, UseProbe,
};
use crate::structs::ghost::{GhostBuild, GhostView};

// Decides what each use call does while the player holds the use key over a ghost build. The shim
// calls `aim`, then `decide`, once per use call and carries out the returned command.
//
// Bounds: the pending set holds `pending_capacity` entries and evicts the oldest, which counts as
// rejected. The tried set holds `tried_capacity` spots and drops the oldest. The per-hold notice
// list and the half-filled double slab list share the tried capacity and drop the oldest.
pub struct EasyPlaceService {
    config: EasyPlaceConfig,
    aim: Aim,
    hold: HoldState,
    pending: PendingSet,
    tried: TriedSet,
    // Double slab spots whose first half was confirmed and whose second half is still to come.
    halved: TriedSet,
    scratch: PendingPlace,
    notice: Option<EasyPlaceNotice>,
    telemetry: EasyPlaceTelemetry,
}

impl EasyPlaceService {
    // How long the HUD keeps a notice, counted on the frame clock from the tick that took it.
    pub const NOTICE_SECONDS: f64 = 3.0;
    // Hotbar slots are 0 to 8 and the main inventory is 9 to 35.
    const HOTBAR_SLOTS: usize = 9;
    const INVENTORY_SLOTS: usize = 36;
    const AIR: &'static str = "minecraft:air";
    // Added to the hit distance so that a rounding tie with the shim's walk never ends the ghost walk
    // before the hit cell. The hit cell itself is excluded by cell, not by distance.
    const HIT_SLACK: f64 = 1e-6;

    pub fn new(config: EasyPlaceConfig) -> Self {
        let pending = PendingSet::new(config.pending_capacity());
        let tried = TriedSet::new(config.tried_capacity());
        let hold = HoldState::new(config.tried_capacity());
        let halved = TriedSet::new(config.tried_capacity());
        Self {
            config,
            aim: Aim::new(),
            hold,
            pending,
            tried,
            halved,
            scratch: PendingPlace::new(),
            notice: None,
            telemetry: EasyPlaceTelemetry::default(),
        }
    }

    pub fn reconfigure(&mut self, config: EasyPlaceConfig) {
        let evicted = self.pending.set_capacity(config.pending_capacity());
        self.telemetry.rejected += evicted as u64;
        self.tried.set_capacity(config.tried_capacity());
        self.hold.set_capacity(config.tried_capacity());
        self.halved.set_capacity(config.tried_capacity());
        self.config = config;
    }

    // Step one of a call. Returns the spot the shim must read `SpotWorld` for, or None. `build` is
    // None while easy place is off, the ghost is hidden or it is not ready.
    pub fn aim(&mut self, probe: &UseProbe, build: Option<(&GhostBuild, GhostView, &PlaceNeeds)>) -> Option<[i32; 3]> {
        self.expire(probe.time_seconds);
        self.aim.kind = AimKind::Off;
        if probe.game_mode == GameMode::Other {
            return None;
        }
        let (build, view, needs) = build?;
        if let Some((_, started)) = self.hold.flight
            && probe.swap_answer == SwapAnswer::None
            && probe.time_seconds < started + self.config.swap_timeout_seconds()
        {
            self.aim.kind = AimKind::Waiting;
            return None;
        }
        let limits = match probe.hit {
            Some(hit) if !hit.replaceable => RayLimits::new(
                probe.reach.min(hit.distance + Self::HIT_SLACK),
                Some(hit.cell),
                self.halved.contains(hit.cell),
                probe.body,
            ),
            _ => RayLimits::new(probe.reach, None, false, probe.body),
        };
        self.aim.kind = AimKind::Clear;
        let (spot, palette_id) = GhostRay::first_spot(build, view, needs, probe.eye, probe.dir, &limits)?;
        let PlaceNeed::Place { item, rule, slab_of, half } = needs.need(palette_id) else {
            return None;
        };
        if self.pending.contains(spot) || self.tried.contains(spot) {
            self.aim.kind = AimKind::Known;
            return None;
        }
        let expected = needs.block_name(palette_id).unwrap_or("");
        self.aim.set_target(spot, palette_id, *rule, *half);
        self.aim.set_names(&item.0, expected, slab_of.as_deref().unwrap_or(""));
        self.aim.kind = AimKind::Spot;
        Some(spot)
    }

    // Step two of a call. `world` is Some exactly when `aim` returned a spot.
    pub fn decide(
        &mut self,
        probe: &UseProbe,
        world: Option<&SpotWorld>,
        inventory: &[InventorySlot<'_>],
        selected: u8,
    ) -> UseCommand {
        if self.aim.kind != AimKind::Off
            && let Some(command) = self.resolve_flight(probe)
        {
            return command;
        }
        match self.aim.kind {
            AimKind::Off => UseCommand::Vanilla,
            AimKind::Waiting => UseCommand::Block,
            AimKind::Clear => {
                if self.hold.active {
                    self.telemetry.blocked_vanilla += 1;
                    UseCommand::Block
                } else {
                    UseCommand::Vanilla
                }
            }
            AimKind::Known => {
                self.hold.active = true;
                self.telemetry.blocked_vanilla += 1;
                UseCommand::Block
            }
            AimKind::Spot => {
                self.hold.active = true;
                match world {
                    Some(world) => self.place(probe, world, inventory, usize::from(selected)),
                    None => UseCommand::Block,
                }
            }
        }
    }

    // A block changed in the world.
    pub fn seen(&mut self, cell: [i32; 3], block: &[u8], _time_seconds: f64) {
        self.pending.record_seen(cell, block);
    }

    // The shim did not carry out the placement at `spot`: no HUD to select with, a select the game did not
    // take, or a build the game refused. The pending entry goes and counts as not sent instead of placed.
    pub fn not_sent(&mut self, spot: [i32; 3]) {
        if self.pending.remove(spot) {
            self.telemetry.placed = self.telemetry.placed.saturating_sub(1);
            self.telemetry.not_sent += 1;
        }
    }

    // The use key went up, or a gate closed. The pending set stays.
    pub fn end_hold(&mut self) {
        self.hold.clear();
        self.tried.clear();
        self.aim.kind = AimKind::Off;
    }

    // Holds the latest notice only. A newer notice replaces an older one, so the caller reads it once
    // per call.
    pub fn take_notice(&mut self) -> Option<EasyPlaceNotice> {
        self.notice.take()
    }

    pub fn telemetry(&self) -> EasyPlaceTelemetry {
        self.telemetry
    }

    // Placements that wait for the server.
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    // Spots tried in this hold.
    pub fn tried_len(&self) -> usize {
        self.tried.len()
    }

    // Counts every pending entry whose deadline has passed.
    pub fn expire(&mut self, time_seconds: f64) {
        while self.pending.pop_due(time_seconds, &mut self.scratch) {
            let seen = self.scratch.seen.as_str();
            if seen.is_empty() || seen == Self::AIR {
                self.telemetry.rejected += 1;
            } else if seen == self.scratch.expected {
                self.telemetry.confirmed += 1;
                self.halved.remove(self.scratch.spot);
            } else if !self.scratch.slab.is_empty() && seen == self.scratch.slab {
                self.telemetry.confirmed += 1;
                if self.scratch.first_half {
                    self.tried.remove(self.scratch.spot);
                    self.halved.push(self.scratch.spot);
                }
            } else {
                self.telemetry.wrong += 1;
            }
        }
    }

    // While a swap or pick waits for the server, every call is blocked until it ends.
    fn resolve_flight(&mut self, probe: &UseProbe) -> Option<UseCommand> {
        let (spot, started) = self.hold.flight?;
        let failed = match probe.swap_answer {
            SwapAnswer::Accepted => {
                self.hold.flight = None;
                return None;
            }
            SwapAnswer::Refused => true,
            SwapAnswer::None => probe.time_seconds >= started + self.config.swap_timeout_seconds(),
        };
        if failed {
            self.hold.flight = None;
            self.tried.push(spot);
        }
        Some(UseCommand::Block)
    }

    fn place(
        &mut self,
        probe: &UseProbe,
        world: &SpotWorld,
        inventory: &[InventorySlot<'_>],
        selected: usize,
    ) -> UseCommand {
        let spot = self.aim.spot;
        let Some(placement) = Self::choose_face(probe, spot, self.aim.rule, self.aim.half, world) else {
            self.notice = Some(EasyPlaceNotice::NoNeighbour);
            self.telemetry.cancelled_no_neighbour += 1;
            self.tried.push(spot);
            return UseCommand::Block;
        };

        let item = self.aim.item.as_bytes();
        if Self::holds(inventory, selected, item) {
            self.record_placement(probe.time_seconds);
            return UseCommand::Place(placement);
        }
        if let Some(slot) = (0..Self::HOTBAR_SLOTS).find(|slot| Self::holds(inventory, *slot, item)) {
            self.record_placement(probe.time_seconds);
            return UseCommand::Select { slot: slot as u8, place: placement };
        }
        if let Some(from) = (Self::HOTBAR_SLOTS..Self::INVENTORY_SLOTS).find(|slot| Self::holds(inventory, *slot, item))
        {
            self.hold.flight = Some((spot, probe.time_seconds));
            return UseCommand::Swap { from: from as u8, to: selected as u8 };
        }

        match probe.game_mode {
            GameMode::Creative => {
                let slot = (0..Self::HOTBAR_SLOTS)
                    .find(|slot| inventory.get(*slot).is_none_or(|stack| stack.count == 0))
                    .unwrap_or(selected);
                self.hold.flight = Some((spot, probe.time_seconds));
                UseCommand::Pick { palette_id: self.aim.palette_id, slot: slot as u8 }
            }
            _ => {
                if self.hold.note(self.aim.palette_id) {
                    self.notice = Some(EasyPlaceNotice::Missing { palette_id: self.aim.palette_id });
                }
                self.telemetry.cancelled_missing += 1;
                self.tried.push(spot);
                UseCommand::Block
            }
        }
    }

    fn holds(inventory: &[InventorySlot<'_>], slot: usize, item: &[u8]) -> bool {
        inventory.get(slot).is_some_and(|stack| stack.count > 0 && stack.item == item)
    }

    // Marks the aimed spot as placed and waiting for the server.
    fn record_placement(&mut self, time_seconds: f64) {
        let deadline = time_seconds + self.config.pending_timeout_seconds();
        let first_half = !self.aim.slab.is_empty() && !self.halved.contains(self.aim.spot);
        if self.pending.push(self.aim.spot, &self.aim.expected, &self.aim.slab, first_half, deadline) {
            self.telemetry.rejected += 1;
        }
        self.tried.push(self.aim.spot);
        self.telemetry.placed += 1;
    }

    // The click point on the neighbour's face that points at the spot, which is `face`: the face centre,
    // moved up or down on a side face so the game picks the wanted half.
    fn click_point(face: Face, half: Half) -> [f32; 3] {
        let normal = face.normal();
        let mut click = [0.5, 0.5, 0.5];
        for axis in 0..3 {
            if normal[axis] != 0 {
                click[axis] = if normal[axis] > 0 { 1.0 } else { 0.0 };
            }
        }
        if normal[1] == 0 {
            click[1] = match half {
                Half::Any => 0.5,
                Half::Top => 0.75,
                Half::Bottom => 0.25,
            };
        }
        click
    }

    // The visible face the rule allows whose centre is nearest the view ray.
    fn choose_face(
        probe: &UseProbe,
        spot: [i32; 3],
        rule: FaceRule,
        half: Half,
        world: &SpotWorld,
    ) -> Option<Placement> {
        let length = (probe.dir[0] * probe.dir[0] + probe.dir[1] * probe.dir[1] + probe.dir[2] * probe.dir[2]).sqrt();
        if !length.is_finite() || length <= 0.0 {
            return None;
        }
        let dir = probe.dir.map(|v| v / length);
        let centre = [0, 1, 2].map(|axis| f64::from(spot[axis]) + 0.5);

        let mut best: Option<(usize, f64)> = None;
        for (index, face) in Face::ALL.iter().enumerate() {
            let allowed = match rule {
                FaceRule::Any => true,
                FaceRule::Floor => *face == Face::Down,
                FaceRule::Wall => !matches!(face, Face::Down | Face::Up),
            };
            let half_allowed = match half {
                Half::Any => true,
                Half::Top => *face != Face::Down,
                Half::Bottom => *face != Face::Up,
            };
            if !allowed || !half_allowed || !world.against[index] {
                continue;
            }
            let normal = face.normal();
            let face_centre = [0, 1, 2].map(|axis| centre[axis] + 0.5 * f64::from(normal[axis]));
            let from_eye = [0, 1, 2].map(|axis| face_centre[axis] - probe.eye[axis]);
            let facing: f64 = (0..3).map(|axis| from_eye[axis] * f64::from(normal[axis])).sum();
            if facing <= 0.0 {
                continue;
            }
            let along: f64 = (0..3).map(|axis| from_eye[axis] * dir[axis]).sum();
            let off_ray: f64 = (0..3).map(|axis| (from_eye[axis] - along * dir[axis]).powi(2)).sum();
            if best.is_none_or(|(_, nearest)| off_ray < nearest) {
                best = Some((index, off_ray));
            }
        }

        let (index, _) = best?;
        let normal = Face::ALL[index].normal();
        Some(Placement {
            spot,
            neighbour: [spot[0] + normal[0], spot[1] + normal[1], spot[2] + normal[2]],
            face: Face::ALL[index].opposite(),
            click: Self::click_point(Face::ALL[index].opposite(), half),
        })
    }
}
