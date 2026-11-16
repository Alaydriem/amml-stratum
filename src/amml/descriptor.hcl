namespace    = "alaydriem"
name         = "stratum"
display_name = "Stratum"
version      = "26.63.4"
author       = "Alaydriem"
config_path  = "config.hcl"

# Escape reaches the game while the library holds Input, so the pause screen still closes the library.
input_passthrough = [27]

claim "Overlay" {
  mode   = "Shared"
  reason = "Shows the Stratum library"
}

claim "WorldDraw" {
  mode   = "Shared"
  reason = "Draws the active marker's structure in the world"
}

claim "Hud" {
  mode   = "Shared"
  reason = "Keys and a card for the active marker while no screen is open"
}

claim "Input" {
  mode   = "Exclusive"
  reason = "Keeps other mods' keys out while the library is open"
}

keybind "open_library" {
  label       = "Open or close the library"
  default     = "F3+S"
  capability  = "Overlay"
  scope       = "Global"
  config_path = "keys.open_library"
}

keybind "hud_up" {
  label       = "HUD up"
  default     = "Up"
  capability  = "Hud"
  scope       = "WhileHeld"
  config_path = "keys.hud_up"
}

keybind "hud_down" {
  label       = "HUD down"
  default     = "Down"
  capability  = "Hud"
  scope       = "WhileHeld"
  config_path = "keys.hud_down"
}

keybind "hud_left" {
  label       = "HUD left"
  default     = "Left"
  capability  = "Hud"
  scope       = "WhileHeld"
  config_path = "keys.hud_left"
}

keybind "hud_right" {
  label       = "HUD right"
  default     = "Right"
  capability  = "Hud"
  scope       = "WhileHeld"
  config_path = "keys.hud_right"
}

keybind "hud_mode" {
  label       = "HUD mode"
  default     = "F10"
  capability  = "Hud"
  scope       = "WhileHeld"
  config_path = "keys.hud_mode"
}

section "keys" {
  title = "Keys"

  field "keys.open_library" {
    label   = "Open or close the library"
    kind    = "chord"
    default = "F3+S"
    apply   = "Live"
  }

  field "keys.hud_up" {
    label   = "HUD up"
    kind    = "chord"
    default = "Up"
    apply   = "Live"
  }

  field "keys.hud_down" {
    label   = "HUD down"
    kind    = "chord"
    default = "Down"
    apply   = "Live"
  }

  field "keys.hud_left" {
    label   = "HUD left"
    kind    = "chord"
    default = "Left"
    apply   = "Live"
  }

  field "keys.hud_right" {
    label   = "HUD right"
    kind    = "chord"
    default = "Right"
    apply   = "Live"
  }

  field "keys.hud_mode" {
    label   = "HUD mode"
    kind    = "chord"
    default = "F10"
    apply   = "Live"
  }
}

section "library" {
  title = "Library"

  field "library.max_structures" {
    label   = "Most structures"
    help    = "Imports past this are refused"
    kind    = { int = { min = 16, max = 65536, step = 1 } }
    default = 4096
    apply   = "Restart"
  }

  field "library.max_projects" {
    label   = "Most projects"
    kind    = { int = { min = 1, max = 4096, step = 1 } }
    default = 512
    apply   = "Restart"
  }

  field "markers.max_per_world" {
    label   = "Most markers per world"
    kind    = { int = { min = 1, max = 4096, step = 1 } }
    default = 256
    apply   = "Restart"
  }
}

section "import" {
  title = "Import"

  field "import.max_file_mb" {
    label   = "Largest file (MB)"
    help    = "Bigger structure files are refused"
    kind    = { int = { min = 1, max = 256, step = 1 } }
    default = 256
    apply   = "Restart"
  }

  field "import.inbox_poll_seconds" {
    label   = "Inbox check interval"
    help    = "Time between checks of the import folder"
    kind    = { float = { min = 0.5, max = 30.0, step = 0.5, unit = "s" } }
    default = 2.0
    apply   = "Restart"
  }
}

section "memory" {
  title = "Memory"

  field "cache.budget_mb" {
    label   = "Structure cache (MB)"
    help    = "Memory for structures read for material lists"
    kind    = { int = { min = 64, max = 2048, step = 1 } }
    default = 128
    apply   = "Restart"
  }

  field "preview.texture_cache" {
    label   = "Preview textures"
    help    = "Block textures the 3D preview keeps decoded"
    kind    = { int = { min = 16, max = 8192, step = 16 } }
    default = 1024
    apply   = "Live"
  }

  field "preview.max_quads" {
    label   = "Preview faces"
    help    = "Most block faces one 3D preview draws; a bigger structure is cut"
    kind    = { int = { min = 10000, max = 4000000, step = 10000 } }
    default = 1000000
    apply   = "Live"
  }
}

section "ghost" {
  title = "Ghost blocks"

  field "ghost.miniature" {
    label   = "Mini ghost"
    help    = "Shows the whole structure in miniature at its lowest corner; off shows a structure block there"
    kind    = "bool"
    default = true
    apply   = "Live"
  }

  field "ghost.draw_distance" {
    label   = "Draw distance (chunks)"
    help    = "0 uses the game's render distance"
    kind    = { int = { min = 0, max = 64, step = 1 } }
    default = 0
    apply   = "Live"
  }

  field "ghost.mesh_budget_mb" {
    label   = "Mesh memory (MB)"
    help    = "Graphics memory the ghost blocks may use"
    kind    = { int = { min = 64, max = 8192, step = 64 } }
    default = 1024
    apply   = "Restart"
  }

  field "ghost.cells_per_frame" {
    label   = "Cells built per frame"
    help    = "Higher builds faster and costs more frame time"
    kind    = { int = { min = 64, max = 2000, step = 1 } }
    default = 600
    apply   = "Live"
  }
}

section "debug" {
  title = "Debug"

  field "debug.telemetry" {
    label   = "Telemetry"
    help    = "Logs tick and frame cost, thread wake-ups and saves every 2 s"
    kind    = "bool"
    default = false
    apply   = "Live"
  }
}
