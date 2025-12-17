-- Kaos.io - Agar.io-style multiplayer game
-- Server-side game logic powered by KaosNet Lua runtime

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
local function spawn_food(state, count)
    for i = 1, count do
        local id = state.next_food_id
        state.next_food_id = id + 1
        state.food[tostring(id)] = {
            id = id,
            x = math.random(50, WORLD_WIDTH - 50),
            y = math.random(50, WORLD_HEIGHT - 50),
            radius = FOOD_RADIUS,
            color = random_color(),
        }
    end
end

-- Process a single message
local function process_message(state, msg)
    local player_id = tostring(msg.sender.session_id)
    local player = state.players[player_id]

    if not player or not player.alive then return end

    -- Parse message data
    local data = msg.data
    if type(data) == "string" then
        local ok
        ok, data = pcall(json.decode, data)
        if not ok then return end
    end

    if data.type == "move" or data.target_x then
        -- Update player's target position (where mouse is pointing)
        player.target_x = math.max(0, math.min(WORLD_WIDTH, data.target_x or data.x or player.x))
        player.target_y = math.max(0, math.min(WORLD_HEIGHT, data.target_y or data.y or player.y))
        -- Update name if provided
        if data.name and #data.name > 0 then
            player.name = data.name
        end

    elseif data.type == "split" then
        -- Split the player (if large enough)
        if player.mass >= MIN_SPLIT_SIZE * 2 then
            player.mass = player.mass / 2

            local dx = player.target_x - player.x
            local dy = player.target_y - player.y
            local len = math.sqrt(dx * dx + dy * dy)
            if len > 0 then
                dx, dy = dx / len, dy / len
            else
                dx, dy = 1, 0
            end

            local eject_dist = mass_to_radius(player.mass) * 2
            local id = state.next_food_id
            state.next_food_id = id + 1
            state.food[tostring(id)] = {
                id = id,
                x = player.x + dx * eject_dist,
                y = player.y + dy * eject_dist,
                radius = FOOD_RADIUS * 2,
                color = player.color,
                value = player.mass / 4,
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
            local id = state.next_food_id
            state.next_food_id = id + 1
            state.food[tostring(id)] = {
                id = id,
                x = player.x + dx * eject_dist,
                y = player.y + dy * eject_dist,
                radius = FOOD_RADIUS * 1.5,
                color = player.color,
            }
        end
    end
end

-- Register the match handler with KaosNet
kaos.register_match("kaos_io", {
    -- Initialize match
    match_init = function(ctx, params)
        kaos.logger_info("Kaos.io match initializing...")

        local state = {
            players = {},
            food = {},
            next_food_id = 1,
            tick_count = 0,
            world_width = WORLD_WIDTH,
            world_height = WORLD_HEIGHT,
        }

        -- Spawn initial food
        spawn_food(state, params.food_count or FOOD_COUNT)

        kaos.logger_info("Kaos.io initialized with " .. FOOD_COUNT .. " food items")

        return {
            state = state,
            tick_rate = 20,  -- 20 ticks per second
            label = "kaos-io-arena",
            max_size = 50,
        }
    end,

    -- Player joins the match
    match_join = function(ctx, state, presences)
        for _, presence in ipairs(presences) do
            local player_id = tostring(presence.session_id)
            local spawn = random_position()

            state.players[player_id] = {
                id = presence.session_id,
                user_id = presence.user_id,
                name = presence.username or ("Player " .. player_id),
                x = spawn.x,
                y = spawn.y,
                target_x = spawn.x,
                target_y = spawn.y,
                mass = radius_to_mass(BASE_RADIUS),
                color = random_color(),
                score = 0,
                kills = 0,
                high_score = 0,
                alive = true,
            }

            kaos.logger_info("Player joined: " .. state.players[player_id].name .. " (session " .. player_id .. ")")
        end

        return state
    end,

    -- Player leaves the match
    match_leave = function(ctx, state, presences)
        for _, presence in ipairs(presences) do
            local player_id = tostring(presence.session_id)
            local player = state.players[player_id]

            if player then
                kaos.logger_info("Player left: " .. player.name .. " (score: " .. player.score .. ")")

                -- Drop some food where player was
                local food_to_drop = math.floor(player.mass / FOOD_VALUE / 2)
                for i = 1, math.min(food_to_drop, 20) do
                    local id = state.next_food_id
                    state.next_food_id = id + 1
                    state.food[tostring(id)] = {
                        id = id,
                        x = player.x + math.random(-50, 50),
                        y = player.y + math.random(-50, 50),
                        radius = FOOD_RADIUS,
                        color = player.color,
                    }
                end

                -- Submit to leaderboard if services available
                if kaos.leaderboard_submit then
                    kaos.leaderboard_submit("kaos_io_highscores", player.user_id, player.name, player.score, {
                        kills = player.kills,
                        color = player.color,
                    })
                end

                state.players[player_id] = nil
            end
        end

        return state
    end,

    -- Main game tick
    match_loop = function(ctx, state, tick, messages)
        local dt = 1.0 / 20  -- 20 ticks per second
        state.tick_count = tick

        -- Process all player messages
        for _, msg in ipairs(messages or {}) do
            process_message(state, msg)
        end

        -- Update all players
        local alive_players = {}
        for player_id, player in pairs(state.players) do
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
                    player.x = player.x + (dx / dist) * speed * dt
                    player.y = player.y + (dy / dist) * speed * dt
                end

                -- Clamp to world bounds
                player.x = math.max(radius, math.min(WORLD_WIDTH - radius, player.x))
                player.y = math.max(radius, math.min(WORLD_HEIGHT - radius, player.y))

                -- Mass decay for large players
                if player.mass > radius_to_mass(BASE_RADIUS * 2) then
                    player.mass = player.mass * (1 - DECAY_RATE * dt)
                end

                -- Update score based on mass
                player.score = math.floor(player.mass)
                if player.score > player.high_score then
                    player.high_score = player.score
                end
            end
        end

        -- Check food collisions
        local eaten_food = {}
        for player_id, player in pairs(state.players) do
            if player.alive then
                local radius = mass_to_radius(player.mass)

                for food_id, food in pairs(state.food) do
                    if not eaten_food[food_id] then
                        local dist = distance(player.x, player.y, food.x, food.y)
                        if dist < radius - food.radius * 0.5 then
                            player.mass = player.mass + (food.value or FOOD_VALUE)
                            eaten_food[food_id] = true
                        end
                    end
                end
            end
        end

        -- Remove eaten food
        for food_id in pairs(eaten_food) do
            state.food[food_id] = nil
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
                            p1.kills = p1.kills + 1
                            p2.alive = false
                            kaos.logger_info(p1.name .. " ate " .. p2.name .. "!")

                        elseif dist < r2 - r1 * 0.4 and p2.mass > p1.mass * 1.1 then
                            -- p2 eats p1
                            p2.mass = p2.mass + p1.mass * 0.8
                            p2.kills = p2.kills + 1
                            p1.alive = false
                            kaos.logger_info(p2.name .. " ate " .. p1.name .. "!")
                        end
                    end
                end
            end
        end

        -- Respawn dead players
        for player_id, player in pairs(state.players) do
            if not player.alive then
                local spawn = random_position()
                player.x = spawn.x
                player.y = spawn.y
                player.mass = radius_to_mass(BASE_RADIUS)
                player.score = 0
                player.alive = true
            end
        end

        -- Respawn food to maintain count
        local food_count = 0
        for _ in pairs(state.food) do food_count = food_count + 1 end
        if food_count < FOOD_COUNT then
            spawn_food(state, math.min(5, FOOD_COUNT - food_count))
        end

        -- Build broadcast-ready state (arrays instead of tables)
        -- This format is expected by the web client
        local players_array = {}
        for player_id, player in pairs(state.players) do
            if player.alive then
                local radius = mass_to_radius(player.mass)
                table.insert(players_array, {
                    id = player.id,
                    name = player.name,
                    x = player.x,
                    y = player.y,
                    radius = radius,
                    color = player.color,
                    score = player.score,
                    high_score = player.high_score or 0,
                    kills = player.kills or 0,
                })
            end
        end

        local food_array = {}
        for food_id, food in pairs(state.food) do
            table.insert(food_array, {
                id = food.id,
                x = food.x,
                y = food.y,
                radius = food.radius,
                color = food.color,
            })
        end

        -- Store broadcast state for the server to send to clients
        state.broadcast = {
            players = players_array,
            food = food_array,
            world_width = WORLD_WIDTH,
            world_height = WORLD_HEIGHT,
            tick = tick,
        }

        return state
    end,

    -- Match terminates
    match_terminate = function(ctx, state)
        kaos.logger_info("Kaos.io match terminated")
    end
})

kaos.logger_info("Kaos.io game module loaded")
