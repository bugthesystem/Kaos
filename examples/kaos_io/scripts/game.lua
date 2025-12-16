-- Kaos.io - Agar.io-style multiplayer game
-- Server-side game logic powered by KaosNet

local json = require("json")

-- Game constants
local WORLD_WIDTH = 2000
local WORLD_HEIGHT = 2000
local BASE_SPEED = 200
local BASE_RADIUS = 20
local FOOD_COUNT = 200
local FOOD_RADIUS = 8
local FOOD_VALUE = 5
local MIN_SPLIT_SIZE = 40
local DECAY_RATE = 0.001  -- Mass decay per second for large players

-- Game state
local state = {
    players = {},      -- player_id -> player data
    food = {},         -- food items
    tick_count = 0,
    last_food_spawn = 0,
}

-- Utility functions
local function random_color()
    local colors = {
        "#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4", "#FFEAA7",
        "#DDA0DD", "#98D8C8", "#F7DC6F", "#BB8FCE", "#85C1E9",
        "#F8B500", "#FF6F61", "#6B5B95", "#88B04B", "#F7CAC9",
    }
    return colors[math.random(#colors)]
end

local function random_position()
    return {
        x = math.random(100, WORLD_WIDTH - 100),
        y = math.random(100, WORLD_HEIGHT - 100),
    }
end

local function distance(x1, y1, x2, y2)
    local dx = x2 - x1
    local dy = y2 - y1
    return math.sqrt(dx * dx + dy * dy)
end

local function mass_to_radius(mass)
    return math.sqrt(mass / math.pi) * 4
end

local function radius_to_mass(radius)
    return math.pi * (radius / 4) ^ 2
end

-- Spawn food at random positions
local function spawn_food(count)
    for i = 1, count do
        local id = #state.food + 1
        state.food[id] = {
            id = id,
            x = math.random(50, WORLD_WIDTH - 50),
            y = math.random(50, WORLD_HEIGHT - 50),
            radius = FOOD_RADIUS,
            color = random_color(),
        }
    end
end

-- Initialize match
function match_init(params)
    kaos.log("Kaos.io match initializing...")

    -- Parse params if provided
    local config = {}
    if params and #params > 0 then
        local ok, parsed = pcall(json.decode, params)
        if ok then config = parsed end
    end

    -- Initialize world
    state.players = {}
    state.food = {}
    state.tick_count = 0

    -- Spawn initial food
    spawn_food(config.food_count or FOOD_COUNT)

    kaos.log("Kaos.io initialized with " .. #state.food .. " food items")

    return json.encode(state)
end

-- Player joins the match
function match_join(state_data, presences)
    local current = json.decode(state_data)

    for _, presence in ipairs(presences) do
        local player_id = tostring(presence.session_id)
        local spawn = random_position()

        current.players[player_id] = {
            id = player_id,
            name = presence.username or ("Player " .. player_id),
            x = spawn.x,
            y = spawn.y,
            vx = 0,
            vy = 0,
            target_x = spawn.x,
            target_y = spawn.y,
            mass = radius_to_mass(BASE_RADIUS),
            color = random_color(),
            score = 0,
            alive = true,
        }

        kaos.log("Player joined: " .. current.players[player_id].name)

        -- Notify all players about new player
        kaos.broadcast(json.encode({
            type = "player_joined",
            player = current.players[player_id],
        }))
    end

    return json.encode(current)
end

-- Player leaves the match
function match_leave(state_data, presences)
    local current = json.decode(state_data)

    for _, presence in ipairs(presences) do
        local player_id = tostring(presence.session_id)
        local player = current.players[player_id]

        if player then
            kaos.log("Player left: " .. player.name)

            -- Drop some food where player was
            local food_to_drop = math.floor(player.mass / FOOD_VALUE / 2)
            for i = 1, math.min(food_to_drop, 20) do
                local id = #state.food + 1
                current.food[id] = {
                    id = id,
                    x = player.x + math.random(-50, 50),
                    y = player.y + math.random(-50, 50),
                    radius = FOOD_RADIUS,
                    color = player.color,
                }
            end

            current.players[player_id] = nil

            -- Notify all players
            kaos.broadcast(json.encode({
                type = "player_left",
                player_id = player_id,
            }))
        end
    end

    return json.encode(current)
end

-- Process a single message
local function process_message(current, msg)
    local player_id = tostring(msg.sender)
    local player = current.players[player_id]

    if not player or not player.alive then return end

    -- Parse message data
    local ok, data = pcall(json.decode, msg.data)
    if not ok then return end

    if data.type == "move" then
        -- Update player's target position (where mouse is pointing)
        player.target_x = math.max(0, math.min(WORLD_WIDTH, data.x or player.x))
        player.target_y = math.max(0, math.min(WORLD_HEIGHT, data.y or player.y))

    elseif data.type == "split" then
        -- Split the player (if large enough)
        if player.mass >= MIN_SPLIT_SIZE * 2 then
            -- Halve the mass
            player.mass = player.mass / 2

            -- Calculate split direction
            local dx = player.target_x - player.x
            local dy = player.target_y - player.y
            local len = math.sqrt(dx * dx + dy * dy)
            if len > 0 then
                dx, dy = dx / len, dy / len
            else
                dx, dy = 1, 0
            end

            -- Create a new "cell" as ejected mass (simplified - just eject as food)
            local eject_dist = mass_to_radius(player.mass) * 2
            local id = #current.food + 1
            current.food[id] = {
                id = id,
                x = player.x + dx * eject_dist,
                y = player.y + dy * eject_dist,
                radius = FOOD_RADIUS * 2,
                color = player.color,
                value = player.mass / 4,  -- Quarter of split mass
            }
        end

    elseif data.type == "eject" then
        -- Eject some mass
        if player.mass > BASE_RADIUS * 2 then
            player.mass = player.mass - FOOD_VALUE * 2

            local dx = player.target_x - player.x
            local dy = player.target_y - player.y
            local len = math.sqrt(dx * dx + dy * dy)
            if len > 0 then
                dx, dy = dx / len, dy / len
            else
                dx, dy = 1, 0
            end

            local eject_dist = mass_to_radius(player.mass) + 20
            local id = #current.food + 1
            current.food[id] = {
                id = id,
                x = player.x + dx * eject_dist,
                y = player.y + dy * eject_dist,
                radius = FOOD_RADIUS * 1.5,
                color = player.color,
            }
        end
    end
end

-- Main game tick
function match_tick(state_data, tick, messages)
    local current = json.decode(state_data)
    local dt = 1.0 / 20  -- 20 ticks per second
    local broadcasts = {}

    current.tick_count = tick

    -- Process all player messages
    for _, msg in ipairs(messages or {}) do
        process_message(current, msg)
    end

    -- Update all players
    local alive_players = {}
    for player_id, player in pairs(current.players) do
        if player.alive then
            table.insert(alive_players, player)

            -- Calculate movement towards target
            local dx = player.target_x - player.x
            local dy = player.target_y - player.y
            local dist = math.sqrt(dx * dx + dy * dy)

            -- Speed decreases with size
            local radius = mass_to_radius(player.mass)
            local speed = BASE_SPEED * (BASE_RADIUS / (radius * 0.5 + BASE_RADIUS * 0.5))

            if dist > 5 then
                -- Normalize and apply speed
                player.vx = (dx / dist) * speed
                player.vy = (dy / dist) * speed
            else
                player.vx = 0
                player.vy = 0
            end

            -- Apply velocity
            player.x = player.x + player.vx * dt
            player.y = player.y + player.vy * dt

            -- Clamp to world bounds
            player.x = math.max(radius, math.min(WORLD_WIDTH - radius, player.x))
            player.y = math.max(radius, math.min(WORLD_HEIGHT - radius, player.y))

            -- Mass decay for large players
            if player.mass > radius_to_mass(BASE_RADIUS * 2) then
                player.mass = player.mass * (1 - DECAY_RATE * dt)
            end
        end
    end

    -- Check food collisions
    local eaten_food = {}
    for player_id, player in pairs(current.players) do
        if player.alive then
            local radius = mass_to_radius(player.mass)

            for food_id, food in pairs(current.food) do
                if not eaten_food[food_id] then
                    local dist = distance(player.x, player.y, food.x, food.y)
                    if dist < radius - food.radius * 0.5 then
                        -- Eat the food
                        player.mass = player.mass + (food.value or FOOD_VALUE)
                        player.score = player.score + 1
                        eaten_food[food_id] = true
                    end
                end
            end
        end
    end

    -- Remove eaten food
    for food_id in pairs(eaten_food) do
        current.food[food_id] = nil
    end

    -- Check player-player collisions
    for i = 1, #alive_players do
        local p1 = alive_players[i]
        if p1.alive then
            local r1 = mass_to_radius(p1.mass)

            for j = i + 1, #alive_players do
                local p2 = alive_players[j]
                if p2.alive then
                    local r2 = mass_to_radius(p2.mass)
                    local dist = distance(p1.x, p1.y, p2.x, p2.y)

                    -- Check if one can eat the other (must be 10% larger)
                    if dist < r1 - r2 * 0.4 and p1.mass > p2.mass * 1.1 then
                        -- p1 eats p2
                        p1.mass = p1.mass + p2.mass * 0.8
                        p1.score = p1.score + p2.score + 10
                        p2.alive = false

                        kaos.log(p1.name .. " ate " .. p2.name .. "!")

                        table.insert(broadcasts, {
                            target = "all",
                            data = json.encode({
                                type = "player_eaten",
                                eater = p1.id,
                                eaten = p2.id,
                                eater_name = p1.name,
                                eaten_name = p2.name,
                            })
                        })

                    elseif dist < r2 - r1 * 0.4 and p2.mass > p1.mass * 1.1 then
                        -- p2 eats p1
                        p2.mass = p2.mass + p1.mass * 0.8
                        p2.score = p2.score + p1.score + 10
                        p1.alive = false

                        kaos.log(p2.name .. " ate " .. p1.name .. "!")

                        table.insert(broadcasts, {
                            target = "all",
                            data = json.encode({
                                type = "player_eaten",
                                eater = p2.id,
                                eaten = p1.id,
                                eater_name = p2.name,
                                eaten_name = p1.name,
                            })
                        })
                    end
                end
            end
        end
    end

    -- Respawn dead players
    for player_id, player in pairs(current.players) do
        if not player.alive then
            -- Respawn after being eaten
            local spawn = random_position()
            player.x = spawn.x
            player.y = spawn.y
            player.mass = radius_to_mass(BASE_RADIUS)
            player.score = 0
            player.alive = true

            table.insert(broadcasts, {
                target = "only",
                targets = {tonumber(player_id)},
                data = json.encode({
                    type = "respawn",
                    player = player,
                })
            })
        end
    end

    -- Respawn food to maintain count
    local food_count = 0
    for _ in pairs(current.food) do food_count = food_count + 1 end
    if food_count < FOOD_COUNT then
        local to_spawn = math.min(5, FOOD_COUNT - food_count)  -- Spawn up to 5 per tick
        for i = 1, to_spawn do
            local id = tick * 1000 + i  -- Unique ID
            current.food[id] = {
                id = id,
                x = math.random(50, WORLD_WIDTH - 50),
                y = math.random(50, WORLD_HEIGHT - 50),
                radius = FOOD_RADIUS,
                color = random_color(),
            }
        end
    end

    -- Build leaderboard
    local leaderboard = {}
    for player_id, player in pairs(current.players) do
        if player.alive then
            table.insert(leaderboard, {
                id = player_id,
                name = player.name,
                score = math.floor(player.mass),
            })
        end
    end
    table.sort(leaderboard, function(a, b) return a.score > b.score end)

    -- Broadcast game state to all players
    table.insert(broadcasts, {
        target = "all",
        data = json.encode({
            type = "state",
            tick = tick,
            players = current.players,
            food = current.food,
            leaderboard = leaderboard,
            world = {
                width = WORLD_WIDTH,
                height = WORLD_HEIGHT,
            }
        })
    })

    return json.encode(current), broadcasts
end

-- Match terminates
function match_terminate(state_data)
    kaos.log("Kaos.io match terminated")
end
