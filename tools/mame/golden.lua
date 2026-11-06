-- Arrête MAME après N images (variable d'environnement GOLDEN_FRAMES, défaut 700).
-- Écrit aussi, si GOLDEN_VIDEO est défini, les pixels bruts de chaque image dans ce fichier (BGRA 32 bits, 768x224).
local frames = tonumber(os.getenv("GOLDEN_FRAMES") or "700")
local video_path = os.getenv("GOLDEN_VIDEO")
local inputs_path = os.getenv("GOLDEN_INPUTS")
local count = 0
local vf = nil
if video_path and video_path ~= "" then vf = assert(io.open(video_path, "wb")) end

-- Table d'entrées : liste triée de {frame, in0, in1, in2} lue dans le fichier d'entrées.
local script = {}
if inputs_path and inputs_path ~= "" then
  for line in io.lines(inputs_path) do
    if not line:match("^%s*#") and line:match("%S") then
      local f, a, b, c = line:match("^%s*(%d+)%s+(%x+)%s+(%x+)%s+(%x+)")
      script[#script + 1] = { tonumber(f), tonumber(a, 16), tonumber(b, 16), tonumber(c, 16) }
    end
  end
end
table.sort(script, function(x, y) return x[1] < y[1] end)

local function state_at(n)
  local cur = { 0, 0, 0x04 }
  for _, e in ipairs(script) do
    if e[1] <= n then cur = { e[2], e[3], e[4] } else break end
  end
  return cur
end

local ioport = manager.machine.ioport
-- Pose les bits d'un port : pour chaque champ dont le masque est dans `mask`, valeur 1 si le bit est actif.
local function apply_port(tag, value, mask)
  local port = ioport.ports[tag]
  if not port then return end
  for _, field in pairs(port.fields) do
    if (field.mask & mask) ~= 0 then
      field:set_value(((value & field.mask) ~= 0) and 1 or 0)
    end
  end
end

local function apply_inputs(n)
  local s = state_at(n)
  -- IN0 : coin1 (01) coin2 (02) gauche (04) droite (08) tir (10) ; IN1 : start1 (01) start2 (02)
  apply_port(":IN0", s[1], 0x1F)
  apply_port(":IN1", s[2], 0x03)
end

apply_inputs(0)
-- register_frame_done : appeler screen:pixels() depuis add_machine_frame_notifier bloque MAME 0.287 après la 1re image.
emu.register_frame_done(function()
  if vf then
    local screen = manager.machine.screens[":screen"]
    local pixels = screen:pixels()
    vf:write(pixels)
  end
  count = count + 1
  apply_inputs(count)
  if count >= frames then
    if vf then vf:close() end
    manager.machine:exit()
  end
end)
