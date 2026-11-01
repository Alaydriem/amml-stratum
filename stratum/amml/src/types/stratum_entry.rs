use amml_game::EntryId;

// the entries only stratum reads, with the one it reuses from another mod's recipes; the entries every mod shares
// are amml_game::EntryId constants
pub struct StratumEntry;

impl StratumEntry {
    pub const LEVEL_RENDERER_RENDER_LEVEL: EntryId = EntryId::new("LevelRenderer.renderLevel");
    pub const LEVEL_RENDERER_ATLAS_TEXTURE: EntryId = EntryId::new("LevelRenderer.mAtlasTexture");
    pub const FRAME_RENDER_OBJECT_VIEWS: EntryId = EntryId::new("FrameRenderObject.mViews");
    pub const CLIENT_INSTANCE_GET_ITEM_IN_HAND_RENDERER: EntryId = EntryId::new("ClientInstance.getItemInHandRenderer");
    pub const ITEM_IN_HAND_RENDERER_MAT_BLEND_BLOCK: EntryId = EntryId::new("ItemInHandRenderer.mMatBlendBlock");
    pub const TESSELLATOR_BEGIN: EntryId = EntryId::new("Tessellator.begin");
    pub const TESSELLATOR_END: EntryId = EntryId::new("Tessellator.end");
    pub const TESSELLATOR_CLEAR: EntryId = EntryId::new("Tessellator.clear");
    // the anchor of the Tessellator mirror
    pub const TESSELLATOR_TESSELLATING: EntryId = EntryId::new("Tessellator.mTessellating");
    pub const MESH_MOVE_CTOR: EntryId = EntryId::new("Mesh.moveCtor");
    pub const MESH_DTOR: EntryId = EntryId::new("Mesh.dtor");
    pub const MESH_IS_VALID: EntryId = EntryId::new("Mesh.isValid");
    pub const MESH_RENDER_MESH: EntryId = EntryId::new("Mesh.renderMesh");
    // sizeof(mce::Mesh), the check of the Mesh mirror
    pub const MESH_SIZE: EntryId = EntryId::new("Mesh.size");
    pub const MESH_DATA_CLEAR: EntryId = EntryId::new("MeshData.clear");
    pub const MESH_DATA_DTOR: EntryId = EntryId::new("MeshData.dtor");
    pub const MESH_DATA_MOVE_ASSIGN: EntryId = EntryId::new("MeshData.moveAssign");
    pub const BLOCK_TESSELLATOR_CTOR: EntryId = EntryId::new("BlockTessellator.ctor");
    pub const BLOCK_TESSELLATOR_DTOR: EntryId = EntryId::new("BlockTessellator.dtor");
    pub const BLOCK_TESSELLATOR_SET_REGION: EntryId = EntryId::new("BlockTessellator.setRegion");
    pub const BLOCK_TESSELLATOR_TESSELLATE_IN_WORLD: EntryId = EntryId::new("BlockTessellator.tessellateInWorld");
    // the anchor of the BlockTessellator mirror
    pub const BLOCK_TESSELLATOR_BLOCK_CACHE: EntryId = EntryId::new("BlockTessellator.mBlockCache");
    pub const BLOCK_TESSELLATOR_CACHE_GET_BLOCK: EntryId = EntryId::new("BlockTessellatorCache.getBlock");
    pub const ACTOR_SHADER_MANAGER_SET_ENTITY_CONSTANTS: EntryId =
        EntryId::new("ActorShaderManager.setEntityConstants");
    pub const GLOBAL_CONSTANT_BUFFERS_ENTITY_CONSTANT_BUFFER: EntryId =
        EntryId::new("GlobalConstantBuffers.entityConstantBuffer");
    pub const MATRIX_STACK_PUSH: EntryId = EntryId::new("MatrixStack.push");
    pub const MATRIX_STACK_REF_DTOR: EntryId = EntryId::new("MatrixStackRef.dtor");

    // the Bedrock::Owner global; the registry object sits REGISTRY_IN_OWNER bytes into it
    pub const BLOCK_TYPE_REGISTRY_INSTANCE: EntryId = EntryId::new("BlockTypeRegistry.instance");
    pub const BLOCK_TYPE_REGISTRY_LOOKUP_BY_NAME: EntryId = EntryId::new("BlockTypeRegistry.lookupByName");
    pub const STRING_BYTE_INPUT_VTABLE: EntryId = EntryId::new("StringByteInput.vtable");
    pub const NBT_IO_READ: EntryId = EntryId::new("NbtIo.read");
    pub const BLOCK_SERIALIZATION_UTILS_TRY_GET_BLOCK_FROM_NBT: EntryId =
        EntryId::new("BlockSerializationUtils.tryGetBlockFromNBT");
    pub const COMPOUND_TAG_DTOR: EntryId = EntryId::new("CompoundTag.dtor");
    pub const BLOCK_GRAPHICS_GET_FOR_BLOCK: EntryId = EntryId::new("BlockGraphics.getForBlock");
    pub const BLOCK_GRAPHICS_RENDER_LAYER: EntryId = EntryId::new("BlockGraphics.mRenderLayer");
    pub const TRANSFORM_BLOCK: EntryId = EntryId::new("VanillaBlockStateTransformUtils.transformBlock");
    pub const HAS_TRANSFORMABLE_WALL_STATES: EntryId =
        EntryId::new("VanillaBlockStateTransformUtils.hasTransformableWallStates");
    pub const TRANSFORM_WALL_CONNECTION_TYPES: EntryId =
        EntryId::new("VanillaBlockStateTransformUtils.transformWallConnectionTypes");
    pub const HAS_TRANSFORMABLE_CONNECTION_STATES: EntryId =
        EntryId::new("VanillaBlockStateTransformUtils.hasTransformableConnectionStates");
    pub const TRANSFORM_CONNECTION_STATES: EntryId =
        EntryId::new("VanillaBlockStateTransformUtils.transformConnectionStates");
    pub const BLOCK_SOURCE_GET_BLOCK: EntryId = EntryId::new("BlockSource.getBlock");
    pub const BIOME_COLOR_SAMPLING_GET_TESSELLATION_POLICY: EntryId =
        EntryId::new("BiomeColorSampling.getTessellationPolicy");
    pub const TESSELLATION_POLICY_GET: EntryId = EntryId::new("TessellationPolicy.get");
    pub const BLOCK_TYPE_TINT_METHOD: EntryId = EntryId::new("BlockType.mTintMethod");

    pub const CLIENT_INSTANCE_GET_LEVEL: EntryId = EntryId::new("ClientInstance.getLevel");
    pub const CLIENT_INSTANCE_IS_IN_REALM: EntryId = EntryId::new("ClientInstance.isInRealm");
    pub const CLIENT_LEVEL_GET_LEVEL_DATA: EntryId = EntryId::new("ClientLevel.getLevelData");
    pub const LEVEL_GET_LEVEL_SEED64: EntryId = EntryId::new("Level.getLevelSeed64");
    pub const LEVEL_DATA_LEVEL_NAME: EntryId = EntryId::new("LevelData.mLevelName");
    pub const I_OPTION_REGISTRY_GET_VIEW_DISTANCE_CHUNKS: EntryId =
        EntryId::new("IOptionRegistry.getViewDistanceChunks");

    // reused from the lightlevels recipes: the `Tessellator&` of a ScreenContext, stored as a pointer
    pub const SCREEN_CONTEXT_TESSELLATOR: EntryId = EntryId::new("ScreenContext.tessellator");

    // reused from the minimap recipes: the connection info and the world storage of the world key
    pub const CLIENT_INSTANCE_GET_GAME_CONNECTION_INFO: EntryId = EntryId::new("ClientInstance.getGameConnectionInfo");
    // sizeof(GameConnectionInfo): the engaged flag of a std::optional<GameConnectionInfo> sits right after it
    pub const GAME_CONNECTION_INFO_SIZE: EntryId = EntryId::new("GameConnectionInfo.size");
    // the non-deleting destructor, `(this) -> void`, run on stratum's own copy of the connection info
    pub const GAME_CONNECTION_INFO_DTOR: EntryId = EntryId::new("GameConnectionInfo.dtor");
    pub const DB_STORAGE_CTOR: EntryId = EntryId::new("DBStorage.ctor");
    pub const DB_STORAGE_CHECK_SHUTDOWN_DONE: EntryId = EntryId::new("DBStorage.checkShutdownDone");

    // reused from the minimap recipes: the block listener on the client region
    pub const BLOCK_SOURCE_LISTENER_VTABLE: EntryId = EntryId::new("BlockSourceListener.vtable");
    pub const BLOCK_SOURCE_LISTENER_ON_SOURCE_DESTROYED: EntryId =
        EntryId::new("BlockSourceListener.onSourceDestroyed");
    pub const BLOCK_SOURCE_LISTENER_ON_BLOCK_CHANGED: EntryId = EntryId::new("BlockSourceListener.onBlockChanged");
    pub const BLOCK_SOURCE_ADD_LISTENER: EntryId = EntryId::new("BlockSource.addListener");
    pub const BLOCK_SOURCE_REMOVE_LISTENER: EntryId = EntryId::new("BlockSource.removeListener");

    // reused from the zoom recipes: the focus loss that opens the pause screen
    pub const MINECRAFT_GAME_ON_APP_FOCUS_LOST: EntryId = EntryId::new("MinecraftGame.onAppFocusLost");

    // easy place: the network handler table and the slots of the three packets it answers
    pub const LEGACY_CLIENT_NETWORK_HANDLER_VTABLE: EntryId = EntryId::new("LegacyClientNetworkHandler.vtable");
    pub const HANDLE_ITEM_STACK_RESPONSE: EntryId = EntryId::new("ClientNetworkHandler.handle.ItemStackResponsePacket");
    pub const HANDLE_CONTAINER_OPEN: EntryId = EntryId::new("ClientNetworkHandler.handle.ContainerOpenPacket");
    pub const HANDLE_CONTAINER_CLOSE: EntryId = EntryId::new("ClientNetworkHandler.handle.ContainerClosePacket");
    // easy place: the use press, the build and the HUD
    pub const CLIENT_INPUT_CALLBACKS_HANDLE_BUILD_ACTION: EntryId =
        EntryId::new("ClientInputCallbacks.handleBuildAction");
    pub const GAME_MODE_VTABLE: EntryId = EntryId::new("GameMode.vtable");
    pub const GAME_MODE_BUILD_BLOCK: EntryId = EntryId::new("GameMode.buildBlock");
    // the anchor of the GameMode mirror
    pub const GAME_MODE_PLAYER: EntryId = EntryId::new("GameMode.mPlayer");
    pub const GAME_MODE_GET_MAX_PICK_RANGE: EntryId = EntryId::new("GameMode.getMaxPickRange");
    pub const HUD_SCREEN_CONTROLLER_VTABLE: EntryId = EntryId::new("HudScreenController.vtable");
    pub const HUD_SCREEN_CONTROLLER_TICK: EntryId = EntryId::new("HudScreenController.tick");
    pub const HUD_SCREEN_CONTROLLER_HANDLE_SLOT_SELECTION: EntryId =
        EntryId::new("HudScreenController.handleSlotSelection");
    pub const SCREEN_CONTROLLER_DTOR: EntryId = EntryId::new("ScreenController.dtor");
    // easy place: the player, the level and the creative items
    pub const CLIENT_INSTANCE_GET_PACKET_SENDER: EntryId = EntryId::new("ClientInstance.getPacketSender");
    pub const PACKET_SENDER_SEND: EntryId = EntryId::new("PacketSender.send");
    pub const ACTOR_GET_VIEW_VECTOR: EntryId = EntryId::new("Actor.getViewVector");
    pub const ACTOR_GET_RUNTIME_ID: EntryId = EntryId::new("Actor.getRuntimeID");
    // reused from the minimap recipes: the actor's collision box
    pub const ACTOR_AABB_SHAPE_COMPONENT: EntryId = EntryId::new("Actor.aabbShapeComponent");
    pub const PLAYER_GET_PLAYER_GAME_TYPE: EntryId = EntryId::new("Player.getPlayerGameType");
    // the anchor of the inventory chain: Player::mInventory, then PlayerInventory and its FillingContainer
    pub const PLAYER_PLAYER_INVENTORY: EntryId = EntryId::new("Player.playerInventory");
    pub const LEVEL_GET_HIT_RESULT: EntryId = EntryId::new("Level.getHitResult");
    pub const LEVEL_GET_ITEM_REGISTRY: EntryId = EntryId::new("Level.getItemRegistry");
    pub const ITEM_REGISTRY_REF_GET_CREATIVE_ITEM_REGISTRY: EntryId =
        EntryId::new("ItemRegistryRef.getCreativeItemRegistry");
    // the anchor of the creative item list
    pub const CREATIVE_ITEM_REGISTRY_CREATIVE_ITEMS: EntryId = EntryId::new("CreativeItemRegistry.mCreativeItems");
    pub const BLOCK_TYPE_CAN_BE_BUILT_OVER: EntryId = EntryId::new("BlockType.canBeBuiltOver");
    pub const BLOCK_TYPE_IS_INTERACTIVE_BLOCK: EntryId = EntryId::new("BlockType.isInteractiveBlock");
    pub const BLOCK_TYPE_AS_ITEM_INSTANCE: EntryId = EntryId::new("BlockType.asItemInstance");
    // easy place: the container and its item stacks
    pub const FILLING_CONTAINER_SWAP_SLOTS: EntryId = EntryId::new("FillingContainer.swapSlots");
    pub const FILLING_CONTAINER_SET_ITEM: EntryId = EntryId::new("FillingContainer.setItem");
    pub const ITEM_STACK_EMPTY_CTOR: EntryId = EntryId::new("ItemStack.emptyCtor");
    pub const ITEM_STACK_COPY_CTOR: EntryId = EntryId::new("ItemStack.copyCtor");
    pub const ITEM_STACK_VTABLE: EntryId = EntryId::new("ItemStack.vtable");
    pub const ITEM_STACK_REINIT: EntryId = EntryId::new("ItemStack.reinit");
    pub const ITEM_STACK_BASE_DTOR: EntryId = EntryId::new("ItemStackBase.dtor");
    pub const ITEM_STACK_BASE_GET_MAX_STACK_SIZE: EntryId = EntryId::new("ItemStackBase.getMaxStackSize");
    // easy place: the packets; the factory and the handler table are reused from the freecam recipes
    pub const MINECRAFT_PACKETS_CREATE_PACKET: EntryId = EntryId::new("MinecraftPackets.createPacket");
    pub const CLIENT_NETWORK_HANDLER_VTABLE: EntryId = EntryId::new("ClientNetworkHandler.vtable");
    pub const PACKET_READ: EntryId = EntryId::new("Packet.read");
    pub const READ_ONLY_BINARY_STREAM_VTABLE: EntryId = EntryId::new("ReadOnlyBinaryStream.vtable");
    pub const READ_ONLY_BINARY_STREAM_DTOR: EntryId = EntryId::new("ReadOnlyBinaryStream.dtor");

    // where the registry object sits in the BlockTypeRegistry.instance global
    pub const REGISTRY_IN_OWNER: u64 = 0x18;

    // easy place: the build hooks, the probe, the inventory and the item stack request
    pub const EASY_PLACE: [EntryId; 42] = [
        Self::LEGACY_CLIENT_NETWORK_HANDLER_VTABLE,
        Self::HANDLE_ITEM_STACK_RESPONSE,
        Self::HANDLE_CONTAINER_OPEN,
        Self::HANDLE_CONTAINER_CLOSE,
        Self::CLIENT_INPUT_CALLBACKS_HANDLE_BUILD_ACTION,
        Self::GAME_MODE_VTABLE,
        Self::GAME_MODE_BUILD_BLOCK,
        Self::GAME_MODE_PLAYER,
        Self::GAME_MODE_GET_MAX_PICK_RANGE,
        Self::HUD_SCREEN_CONTROLLER_VTABLE,
        Self::HUD_SCREEN_CONTROLLER_TICK,
        Self::HUD_SCREEN_CONTROLLER_HANDLE_SLOT_SELECTION,
        Self::SCREEN_CONTROLLER_DTOR,
        Self::CLIENT_INSTANCE_GET_PACKET_SENDER,
        Self::PACKET_SENDER_SEND,
        Self::ACTOR_GET_VIEW_VECTOR,
        EntryId::ACTOR_GET_ATTACH_POS,
        Self::ACTOR_GET_RUNTIME_ID,
        EntryId::ACTOR_LEVEL,
        Self::ACTOR_AABB_SHAPE_COMPONENT,
        Self::PLAYER_GET_PLAYER_GAME_TYPE,
        Self::PLAYER_PLAYER_INVENTORY,
        Self::LEVEL_GET_HIT_RESULT,
        Self::LEVEL_GET_ITEM_REGISTRY,
        Self::ITEM_REGISTRY_REF_GET_CREATIVE_ITEM_REGISTRY,
        Self::CREATIVE_ITEM_REGISTRY_CREATIVE_ITEMS,
        Self::BLOCK_TYPE_CAN_BE_BUILT_OVER,
        Self::BLOCK_TYPE_IS_INTERACTIVE_BLOCK,
        Self::BLOCK_TYPE_AS_ITEM_INSTANCE,
        Self::FILLING_CONTAINER_SWAP_SLOTS,
        Self::FILLING_CONTAINER_SET_ITEM,
        Self::ITEM_STACK_EMPTY_CTOR,
        Self::ITEM_STACK_COPY_CTOR,
        Self::ITEM_STACK_VTABLE,
        Self::ITEM_STACK_REINIT,
        Self::ITEM_STACK_BASE_DTOR,
        Self::ITEM_STACK_BASE_GET_MAX_STACK_SIZE,
        Self::MINECRAFT_PACKETS_CREATE_PACKET,
        Self::CLIENT_NETWORK_HANDLER_VTABLE,
        Self::PACKET_READ,
        Self::READ_ONLY_BINARY_STREAM_VTABLE,
        Self::READ_ONLY_BINARY_STREAM_DTOR,
    ];

    // the ghost build and draw
    pub const GHOST: [EntryId; 27] = [
        Self::LEVEL_RENDERER_RENDER_LEVEL,
        Self::LEVEL_RENDERER_ATLAS_TEXTURE,
        Self::FRAME_RENDER_OBJECT_VIEWS,
        Self::CLIENT_INSTANCE_GET_ITEM_IN_HAND_RENDERER,
        Self::ITEM_IN_HAND_RENDERER_MAT_BLEND_BLOCK,
        Self::TESSELLATOR_BEGIN,
        Self::TESSELLATOR_END,
        Self::TESSELLATOR_CLEAR,
        Self::TESSELLATOR_TESSELLATING,
        Self::MESH_MOVE_CTOR,
        Self::MESH_DTOR,
        Self::MESH_IS_VALID,
        Self::MESH_RENDER_MESH,
        Self::MESH_SIZE,
        Self::MESH_DATA_CLEAR,
        Self::MESH_DATA_DTOR,
        Self::MESH_DATA_MOVE_ASSIGN,
        Self::BLOCK_TESSELLATOR_CTOR,
        Self::BLOCK_TESSELLATOR_DTOR,
        Self::BLOCK_TESSELLATOR_SET_REGION,
        Self::BLOCK_TESSELLATOR_TESSELLATE_IN_WORLD,
        Self::BLOCK_TESSELLATOR_BLOCK_CACHE,
        Self::BLOCK_TESSELLATOR_CACHE_GET_BLOCK,
        Self::ACTOR_SHADER_MANAGER_SET_ENTITY_CONSTANTS,
        Self::GLOBAL_CONSTANT_BUFFERS_ENTITY_CONSTANT_BUFFER,
        Self::MATRIX_STACK_PUSH,
        Self::MATRIX_STACK_REF_DTOR,
    ];

    // the palette lookups, the block turn and the tint
    pub const PALETTE: [EntryId; 17] = [
        Self::BLOCK_TYPE_REGISTRY_INSTANCE,
        Self::BLOCK_TYPE_REGISTRY_LOOKUP_BY_NAME,
        Self::STRING_BYTE_INPUT_VTABLE,
        Self::NBT_IO_READ,
        Self::BLOCK_SERIALIZATION_UTILS_TRY_GET_BLOCK_FROM_NBT,
        Self::COMPOUND_TAG_DTOR,
        Self::BLOCK_GRAPHICS_GET_FOR_BLOCK,
        Self::BLOCK_GRAPHICS_RENDER_LAYER,
        Self::TRANSFORM_BLOCK,
        Self::HAS_TRANSFORMABLE_WALL_STATES,
        Self::TRANSFORM_WALL_CONNECTION_TYPES,
        Self::HAS_TRANSFORMABLE_CONNECTION_STATES,
        Self::TRANSFORM_CONNECTION_STATES,
        Self::BLOCK_SOURCE_GET_BLOCK,
        Self::BIOME_COLOR_SAMPLING_GET_TESSELLATION_POLICY,
        Self::TESSELLATION_POLICY_GET,
        Self::BLOCK_TYPE_TINT_METHOD,
    ];

    // the block listener that tells core which blocks of the client region changed
    pub const BLOCK_CHANGES: [EntryId; 5] = [
        Self::BLOCK_SOURCE_LISTENER_VTABLE,
        Self::BLOCK_SOURCE_LISTENER_ON_SOURCE_DESTROYED,
        Self::BLOCK_SOURCE_LISTENER_ON_BLOCK_CHANGED,
        Self::BLOCK_SOURCE_ADD_LISTENER,
        Self::BLOCK_SOURCE_REMOVE_LISTENER,
    ];

    // the world key of the data folder; the connection info getter and the storage hooks are optional
    pub const WORLD_KEY: [EntryId; 7] = [
        Self::CLIENT_INSTANCE_GET_LEVEL,
        Self::CLIENT_INSTANCE_IS_IN_REALM,
        Self::CLIENT_LEVEL_GET_LEVEL_DATA,
        Self::LEVEL_GET_LEVEL_SEED64,
        Self::LEVEL_DATA_LEVEL_NAME,
        Self::GAME_CONNECTION_INFO_SIZE,
        Self::GAME_CONNECTION_INFO_DTOR,
    ];

    // the frame status of each tick besides the world key, and the focus loss
    pub const STATUS: [EntryId; 6] = [
        EntryId::CLIENT_INSTANCE_IS_SHOWING_MENU,
        EntryId::CLIENT_INSTANCE_GET_OPTIONS,
        Self::I_OPTION_REGISTRY_GET_VIEW_DISTANCE_CHUNKS,
        EntryId::ACTOR_GET_DIMENSION,
        EntryId::DIMENSION_GET_DIMENSION_ID,
        Self::MINECRAFT_GAME_ON_APP_FOCUS_LOST,
    ];

    // the shared entries the ghost reads besides its own: the client, the player's position, the camera mirror
    // anchor and the camera position of a view, and the tessellator of a ScreenContext
    pub const SHARED: [EntryId; 12] = [
        EntryId::CLIENT_INSTANCE_UPDATE,
        EntryId::CLIENT_INSTANCE_GET_LOCAL_PLAYER,
        EntryId::CLIENT_INSTANCE_IS_WORLD_ACTIVE,
        EntryId::CLIENT_INSTANCE_GET_REGION,
        EntryId::ACTOR_STATE_VECTOR_COMPONENT,
        EntryId::ACTOR_ACTOR_ROTATION_COMPONENT,
        EntryId::STATE_VECTOR_COMPONENT_POS,
        EntryId::ACTOR_ROTATION_COMPONENT_ROT,
        EntryId::CAMERA_FORWARD,
        EntryId::VIEW_RENDER_OBJECT_VIEW_DATA,
        EntryId::VIEW_RENDER_DATA_CAMERA_POS,
        Self::SCREEN_CONTEXT_TESSELLATOR,
    ];
}
