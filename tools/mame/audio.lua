-- Joue un sc?nario d'entr?es (GOLDEN_INPUTS) pendant GOLDEN_FRAMES images ; sert ? enregistrer le son MAME (-wavwrite).
local frames = tonumber(os.getenv("GOLDEN_FRAMES") or "1700")
local inputs_path = os.getenv("GOLDEN_INPUTS")
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
  apply_port(":IN0", s[1], 0x1F)
  apply_port(":IN1", s[2], 0x03)
end
apply_inputs(0)
local last_n = -1
emu.register_frame_done(function()
  local n = math.floor(manager.machine.time:as_double() / 0.0165 + 0.5)
  if n == last_n then return end
  last_n = n
  apply_inputs(n)
  if n >= frames then manager.machine:exit() end
end)

