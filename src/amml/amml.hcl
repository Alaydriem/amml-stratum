namespace   = "alaydriem"
name        = "stratum"
entry       = "alaydriem_stratum.dll"
api_version = 3

uses {
  feature "core" {
    event "client.tick" {
      version  = 1
      required = true
    }
    event "world.joined" {
      version  = 1
      required = true
    }
    event "world.left" {
      version  = 1
      required = true
    }
    event "world.dimension_changing" {
      version  = 1
      required = true
    }
    event "world.server_join_starting" {
      version  = 2
      required = true
    }
    event "registry.provider_ready" {
      version  = 1
      required = true
    }
    event "registry.provider_gone" {
      version  = 1
      required = true
    }
    event "mods.loaded" {
      version  = 1
      required = true
    }
    event "input.lease_suspended" {
      version  = 1
      required = true
    }
    event "input.lease_resumed" {
      version  = 1
      required = true
    }
    event "input.lease_lost" {
      version  = 1
      required = true
    }
    event "input.injected_released" {
      version  = 2
      required = true
    }
    event "overlay.screenshot_taken" {
      version  = 1
      required = true
    }
    event "overlay.screenshot_written" {
      version  = 1
      required = true
    }
    area "input.lease" {
      version  = 1
      required = true
    }
    area "client.read" {
      version  = 1
      required = true
    }
    area "client.world_info" {
      version  = 1
      required = true
    }
    area "actor.read" {
      version  = 4
      required = true
    }
    area "options.view_distance" {
      version  = 1
      required = true
    }
  }
  feature "ghost" {
    event "render.world_rendered" {
      version  = 1
      required = true
    }
    area "render.world_canvas" {
      version  = 3
      required = true
    }
    area "render.block_mesher" {
      version  = 1
      required = true
    }
    area "world.block_states" {
      version  = 1
      required = true
    }
    area "render.water_tint" {
      version  = 1
      required = true
    }
  }
  feature "block_changes" {
    event "world.block_changed" {
      version  = 2
      required = true
    }
  }
  feature "block_entities" {
    event "render.block_entity_pass" {
      version  = 1
      required = true
    }
    area "render.block_entities" {
      version  = 1
      required = true
    }
    area "render.chest_pair" {
      version  = 1
      required = true
    }
    area "render.skull_pose" {
      version  = 1
      required = true
    }
  }
  feature "easy_place" {
    event "player.use_pressed" {
      version  = 1
      required = true
    }
    event "player.build_block" {
      version  = 1
      required = true
    }
    event "inventory.request_done" {
      version  = 1
      required = true
    }
    event "input.mouse" {
      version  = 1
      required = true
    }
    event "window.focus" {
      version  = 1
      required = true
    }
    area "world.blocks" {
      version  = 1
      required = true
    }
    area "player.build" {
      version  = 1
      required = true
    }
    area "player.pick_range" {
      version  = 1
      required = true
    }
    area "player.hit_result" {
      version  = 1
      required = true
    }
    area "hud.select_hotbar" {
      version  = 1
      required = true
    }
    area "inventory.read" {
      version  = 3
      required = true
    }
    area "inventory.request" {
      version  = 2
      required = true
    }
  }
  feature "picker" {
    event "client.app_focus_lost" {
      version  = 1
      required = true
    }
  }
  feature "build" {
    area "client.send_command" {
      version  = 1
      required = true
    }
    area "actor.game_type" {
      version  = 1
      required = true
    }
    area "actor.permission" {
      version  = 1
      required = true
    }
    area "world.kinds_in_box" {
      version  = 1
      required = true
    }
  }
}

listing {
  title       = "Stratum"
  summary     = "Import .mcstructure files and build over ghost blocks layer by layer."
  description = <<-EOT
    Litematica for Bedrock. Everything you wished Structura or HoloPrint could be. Import, manage, and render ghost blocks in the game with a powerful F10 menu, with support for big builds, Easy Place, block opacity, structure position and rotation, and cycling through different structures. Stratum lets you see your ghost blocks out to your maximum render distance without the lag, with performance for millions of blocks.
  EOT
  authors     = ["Charles R. Portwood II"]
  license     = "BSD-3-Clause"
  source      = "https://github.com/alaydriem/amml-stratum"
  homepage    = "https://github.com/alaydriem/amml-stratum#readme"
  tags        = ["structures", "mcstructure", "building", "schematic", "litematica"]
  icon        = "icon.png"
}
