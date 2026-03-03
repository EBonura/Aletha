# Ashen Edge — Design Document

## Overview
A PICO-8 action-platformer set inside **The Hollowed Furnace** — a colossal ancient kiln that once burned something divine. The player controls **Aletha**, a tempered figure born of the furnace itself, descending through its chambers to shut it down before it reignites.

## The World — The Hollowed Furnace
The world is not ashen because of disaster. It is ashen because something was **processed** here. This is the inside of an immense kiln — industrial, ancient, mythic. Its chambers are connected by maintenance scaffolds, sealed by heat regulators, and lined with crumbling walls weakened by centuries of thermal stress.

The furnace is dormant but not dead. Red ignition points (torches) keep trying to restart it. The machinery still runs. The constructs still patrol. Everything is waiting for the fire to come back.

**Tone:** Industrial myth. Ancient machinery. Quiet apocalypse.

## Visual Language
- **Red** = residual heat, ignition, the furnace trying to restart
- **Gray** = Aletha, tempered, cooled, finished
- **Black** = ash, processed matter, the kiln's waste
- Torches glow red when active (ignition points), go dark/gray when extinguished (disabled)
- Portals glow red when activated (cooling vents reclaimed)

## Player — Aletha
She was made here. She is the one thing the furnace produced that survived — tempered, neutral, resistant to the heat. She is gray because she already went through the fire. She isn't an outsider coming to save anything. She's the one thing the kiln made that can unmake it.

- Platforming: run, jump
- Two-hit combo: slash → cross-slice
- Can break weakened kiln lining (breakable tiles) by attacking
- Invincibility frames after taking damage
- HP bar (HUD, top-left), gems counter below
- **Zone thoughts** — inner monologue triggered by location, how she pieces together what this place is

## Enemies — Maintenance Constructs
Still running their routines. Not evil — just operational.

### Spiders (type 3)
- Vermin that thrived in the residual heat
- Basic ground enemy, has HP

### Wheelbots (type 4)
- Patrol constructs, still walking their routes
- Sleep state (dormant until disturbed), forward dash attack
- Has HP, invincible during dash

### Hellbots (type 5)
- Heavy-duty furnace guardians
- Tougher, more aggressive

### The Heart of the Kiln (Boss)
- The furnace's core. Not malicious — it's a furnace doing what furnaces do.
- Refuses to cool. Keeps trying to reignite.
- Power tied to active ignition points: more torches extinguished = weaker boss
- Multi-part animated sprite system

## Entities

### Heat Regulators / Switches (type 2)
- Old furnace controls, repurposed by Aletha to open sealed maintenance paths
- Toggled by attacking, cooldown after activation

### Cooling Vents / Portals (type 6)
- Safe spots where the heat can't reach
- Checkpoints — activate by walking near
- Respawn point on death (the heat overtakes her, she retreats to the last cool point)

### Ignition Points / Torches (type 7)
- Residual fire trying to restart the furnace
- Start **lit** (red, animated, 6-frame cycle)
- Extinguished by attacking → static gray frame (disabled)
- Extended hitbox (8px margin) for forgiving interaction
- Core mechanic: each one extinguished weakens the Heart
- Scattered throughout — some obvious, some hidden. Exploration rewarded.

### Thought Zones (type 8)
- Invisible trigger tiles
- Display Aletha's inner monologue when she stands on them
- How the player learns lore, mechanics, and the story
- No UI prompts — everything filtered through her voice

## Gems — Slag
- Cooled remnants of whatever the furnace processed
- Collectible, displayed on HUD
- Purpose: TBD (currency? score? unlock condition?)

## Game Flow
1. **Title screen** → "press x" → fade in
2. **Descent** — explore chambers, fight constructs, extinguish ignition points, find cooling vents
3. **Death** → heat overtakes Aletha → respawn at last cooling vent
4. **The Heart** — final encounter, difficulty scales with remaining ignition points

## Level Structure
- One continuous descent through the furnace (or discrete chambers?)
- Parallax background layer (deep kiln walls)
- Main gameplay layer (scaffolds, platforms, mechanisms)
- Entities placed via level_data.json
- Multi-group system for chamber progression

## Open Questions
- [x] ~~What is the world?~~ → The Hollowed Furnace, an ancient divine kiln
- [x] ~~Who is Aletha?~~ → Tempered by the furnace, the one thing it made that can unmake it
- [x] ~~What is the boss?~~ → The Heart of the Kiln, refusing to cool
- [ ] What do gems/slag do mechanically? Currency? Gate to boss? Ending condition?
- [ ] How exactly does torch count affect the boss fight? (HP? phases? attacks?)
- [ ] Is there an ending? Multiple endings based on torches/slag?
- [ ] One continuous map or discrete chambers?
- [x] ~~What are the zone texts?~~ → Aletha's inner thoughts, reacting to her surroundings
- [ ] What was the furnace burning? Why? Who built it?
