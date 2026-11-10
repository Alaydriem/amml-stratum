namespace   = "alaydriem"
name        = "stratum"
entry       = "alaydriem_stratum.dll"
api_version = 2
amml_version = "26.60.6"
entries = [
  # required: the ghost build and draw
  "LevelRenderer.renderLevel",
  "LevelRenderer.mAtlasTexture",
  "FrameRenderObject.mViews",
  "ClientInstance.getItemInHandRenderer",
  "ItemInHandRenderer.mMatBlendBlock",
  "Tessellator.begin",
  "Tessellator.end",
  "Tessellator.clear",
  "Tessellator.mTessellating",
  "Mesh.moveCtor",
  "Mesh.dtor",
  "Mesh.isValid",
  "Mesh.renderMesh",
  "Mesh.size",
  "MeshData.clear",
  "MeshData.dtor",
  "MeshData.moveAssign",
  "BlockTessellator.ctor",
  "BlockTessellator.dtor",
  "BlockTessellator.setRegion",
  "BlockTessellator.tessellateInWorld",
  "BlockTessellator.mBlockCache",
  "BlockTessellatorCache.getBlock",
  "ActorShaderManager.setEntityConstants",
  "GlobalConstantBuffers.entityConstantBuffer",
  "MatrixStack.push",
  "MatrixStackRef.dtor",
  # required: the palette lookups, the block turn and the tint
  "BlockTypeRegistry.instance",
  "BlockTypeRegistry.lookupByName",
  "StringByteInput.vtable",
  "NbtIo.read",
  "BlockSerializationUtils.tryGetBlockFromNBT",
  "CompoundTag.dtor",
  "BlockGraphics.getForBlock",
  "BlockGraphics.mRenderLayer",
  "VanillaBlockStateTransformUtils.transformBlock",
  "VanillaBlockStateTransformUtils.hasTransformableWallStates",
  "VanillaBlockStateTransformUtils.transformWallConnectionTypes",
  "VanillaBlockStateTransformUtils.hasTransformableConnectionStates",
  "VanillaBlockStateTransformUtils.transformConnectionStates",
  "BlockSource.getBlock",
  "BiomeColorSampling.getTessellationPolicy",
  "TessellationPolicy.get",
  "BlockType.mTintMethod",
  # required: the block listener on the client region
  "BlockSourceListener.vtable",
  "BlockSourceListener.onSourceDestroyed",
  "BlockSourceListener.onBlockChanged",
  "BlockSource.addListener",
  "BlockSource.removeListener",
  # required: the client, the player's position, the camera and the tessellator of a ScreenContext
  "ClientInstance.update",
  "ClientInstance.getLocalPlayer",
  "ClientInstance.isWorldActive",
  "ClientInstance.getRegion",
  "Actor.stateVectorComponent",
  "Actor.actorRotationComponent",
  "StateVectorComponent.mPos",
  "ActorRotationComponent.mRot",
  "Camera.mForward",
  "ViewRenderObject.mViewData",
  "ViewRenderData.mCameraPos",
  "ScreenContext.tessellator",
  # required: the world key of the data folder
  "ClientInstance.getLevel",
  "ClientInstance.isInRealm",
  "ClientLevel.getLevelData",
  "Level.getLevelSeed64",
  "LevelData.mLevelName",
  "GameConnectionInfo.size",
  "GameConnectionInfo.dtor",
  # required: the frame status of each tick and the focus loss
  "ClientInstance.isShowingMenu",
  "ClientInstance.getOptions",
  "IOptionRegistry.getViewDistanceChunks",
  "Actor.getDimension",
  "Dimension.getDimensionId",
  "MinecraftGame.onAppFocusLost",
  # optional, stratum.connection
  "ClientInstance.getGameConnectionInfo",
  # optional, stratum.world_folder
  "DBStorage.ctor",
  "DBStorage.checkShutdownDone",
  # required: easy place, the build hooks, the probe, the inventory and the item stack request
  "LegacyClientNetworkHandler.vtable",
  "ClientNetworkHandler.handle.ItemStackResponsePacket",
  "ClientNetworkHandler.handle.ContainerOpenPacket",
  "ClientNetworkHandler.handle.ContainerClosePacket",
  "ClientInputCallbacks.handleBuildAction",
  "GameMode.vtable",
  "GameMode.buildBlock",
  "GameMode.mPlayer",
  "GameMode.getMaxPickRange",
  "HudScreenController.vtable",
  "HudScreenController.tick",
  "HudScreenController.handleSlotSelection",
  "ScreenController.dtor",
  "ClientInstance.getPacketSender",
  "PacketSender.send",
  "Actor.getViewVector",
  "Actor.getAttachPos",
  "Actor.getRuntimeID",
  "Actor.mLevel",
  "Actor.aabbShapeComponent",
  "Player.getPlayerGameType",
  "Player.playerInventory",
  "Level.getHitResult",
  "Level.getItemRegistry",
  "ItemRegistryRef.getCreativeItemRegistry",
  "CreativeItemRegistry.mCreativeItems",
  "BlockType.canBeBuiltOver",
  "BlockType.isInteractiveBlock",
  "BlockType.asItemInstance",
  "FillingContainer.swapSlots",
  "FillingContainer.setItem",
  "ItemStack.emptyCtor",
  "ItemStack.copyCtor",
  "ItemStack.vtable",
  "ItemStack.reinit",
  "ItemStackBase.dtor",
  "ItemStackBase.getMaxStackSize",
  "MinecraftPackets.createPacket",
  "ClientNetworkHandler.vtable",
  "Packet.read",
  "ReadOnlyBinaryStream.vtable",
  "ReadOnlyBinaryStream.dtor",
]

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
