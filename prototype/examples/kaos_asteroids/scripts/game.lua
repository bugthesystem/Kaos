-- Kaos Asteroids - Server-side game logic
-- All game physics, collisions, and state management in Lua

-- Game constants
local WORLD_WIDTH = 100.0
local WORLD_HEIGHT = 50.0
local SHIP_RADIUS = 1.5
local BULLET_RADIUS = 0.3
local BULLET_SPEED = 50.0
local BULLET_LIFETIME = 2.0
local SHIP_THRUST = 20.0
local SHIP_ROTATION_SPEED = 4.0
local SHIP_DRAG = 0.98
local ASTEROID_SPEED = 8.0
local INITIAL_ASTEROIDS = 5
local RESPAWN_TIME = 3.0
local FIRE_COOLDOWN = 0.2
local LEADERBOARD_ID = "asteroids_highscore"

-- Helper functions
local function distance_squared(x1, y1, x2, y2)
    local dx = x1 - x2
    local dy = y1 - y2
    return dx * dx + dy * dy
end

local function wrap_position(x, y)
    return (x + WORLD_WIDTH) % WORLD_WIDTH, (y + WORLD_HEIGHT) % WORLD_HEIGHT
end

local function asteroid_radius(size)
    if size == 3 then return 3.0
    elseif size == 2 then return 2.0
    else return 1.0 end
end

local function asteroid_points(size)
    if size == 3 then return 20
    elseif size == 2 then return 50
    else return 100 end
end

local function spawn_asteroid(state, size, spawn_x, spawn_y)
    local id = state.next_asteroid_id
    state.next_asteroid_id = id + 1

    local x, y
    if spawn_x and spawn_y then
        x, y = spawn_x, spawn_y
    else
        -- Spawn at edges
        if math.random() < 0.5 then
            x = math.random() * WORLD_WIDTH
            y = math.random() < 0.5 and 0 or WORLD_HEIGHT
        else
            x = math.random() < 0.5 and 0 or WORLD_WIDTH
            y = math.random() * WORLD_HEIGHT
        end
    end

    local angle = math.random() * 2 * math.pi
    local speed = ASTEROID_SPEED * (4 - size) / 3.0

    state.asteroids[tostring(id)] = {
        id = id,
        x = x,
        y = y,
        vx = math.cos(angle) * speed,
        vy = math.sin(angle) * speed,
        size = size,
    }
end

-- Register the match handler with KaosNet
kaos.register_match("kaos_asteroids", {
    -- Initialize match
    match_init = function(ctx, params)
        kaos.logger_info("Kaos Asteroids match initializing...")

        local state = {
            ships = {},
            asteroids = {},
            bullets = {},
            next_asteroid_id = 1,
            next_bullet_id = 1,
            tick_count = 0,
            world_width = WORLD_WIDTH,
            world_height = WORLD_HEIGHT,
        }

        -- Spawn initial asteroids
        for i = 1, INITIAL_ASTEROIDS do
            spawn_asteroid(state, 3)
        end

        kaos.logger_info("Kaos Asteroids initialized with " .. INITIAL_ASTEROIDS .. " asteroids")

        return {
            state = state,
            tick_rate = 60,  -- 60 ticks per second
            label = "asteroids-arena",
            max_size = 20,
        }
    end,

    -- Player joins the match
    match_join = function(ctx, state, presences)
        for _, presence in ipairs(presences) do
            local player_id = tostring(presence.session_id)

            -- Spawn ship at center (offset by player count for spacing)
            local ship_count = 0
            for _ in pairs(state.ships) do ship_count = ship_count + 1 end

            state.ships[player_id] = {
                id = presence.session_id,
                name = presence.username or ("Player" .. presence.session_id),
                user_id = presence.user_id,
                x = WORLD_WIDTH / 2.0 + (ship_count * 5.0) % WORLD_WIDTH,
                y = WORLD_HEIGHT / 2.0,
                vx = 0,
                vy = 0,
                angle = 0,
                score = 0,
                alive = true,
                respawn_timer = 0,
                fire_cooldown = 0,
                input = {
                    thrust = false,
                    left = false,
                    right = false,
                    fire = false,
                },
            }

            kaos.logger_info("Ship joined: " .. state.ships[player_id].name .. " (session " .. player_id .. ")")
        end

        return state
    end,

    -- Player leaves the match
    match_leave = function(ctx, state, presences)
        for _, presence in ipairs(presences) do
            local player_id = tostring(presence.session_id)
            local ship = state.ships[player_id]

            if ship then
                kaos.logger_info("Ship left: " .. ship.name .. " (score: " .. ship.score .. ")")

                -- Submit score to leaderboard via kaos API
                if ship.score > 0 then
                    kaos.leaderboard_submit(
                        LEADERBOARD_ID,
                        ship.user_id or player_id,
                        ship.name,
                        ship.score,
                        { deaths = 0 }
                    )
                end

                -- Remove ship
                state.ships[player_id] = nil
            end
        end

        return state
    end,

    -- Main game tick
    match_loop = function(ctx, state, tick, messages)
        local dt = 1.0 / 60  -- 60 ticks per second
        state.tick_count = tick

        -- Process player input messages
        for _, msg in ipairs(messages or {}) do
            local player_id = tostring(msg.sender.session_id)
            local ship = state.ships[player_id]

            if ship then
                -- Parse input from message data
                local ok, input = pcall(function()
                    return kaos.json_decode(msg.data)
                end)

                if ok and input then
                    ship.input.thrust = input.thrust or false
                    ship.input.left = input.left or false
                    ship.input.right = input.right or false
                    ship.input.fire = input.fire or false
                end
            end
        end

        -- Update ships
        for player_id, ship in pairs(state.ships) do
            if not ship.alive then
                -- Respawn timer
                ship.respawn_timer = ship.respawn_timer - dt
                if ship.respawn_timer <= 0 then
                    ship.alive = true
                    ship.x = WORLD_WIDTH / 2.0
                    ship.y = WORLD_HEIGHT / 2.0
                    ship.vx = 0
                    ship.vy = 0
                end
            else
                -- Rotation
                if ship.input.left then
                    ship.angle = ship.angle - SHIP_ROTATION_SPEED * dt
                end
                if ship.input.right then
                    ship.angle = ship.angle + SHIP_ROTATION_SPEED * dt
                end

                -- Thrust
                if ship.input.thrust then
                    ship.vx = ship.vx + math.cos(ship.angle) * SHIP_THRUST * dt
                    ship.vy = ship.vy + math.sin(ship.angle) * SHIP_THRUST * dt
                end

                -- Fire bullets
                ship.fire_cooldown = ship.fire_cooldown - dt
                if ship.input.fire and ship.fire_cooldown <= 0 then
                    local bullet_id = state.next_bullet_id
                    state.next_bullet_id = bullet_id + 1

                    state.bullets[tostring(bullet_id)] = {
                        id = bullet_id,
                        owner_id = ship.id,
                        x = ship.x + math.cos(ship.angle) * SHIP_RADIUS,
                        y = ship.y + math.sin(ship.angle) * SHIP_RADIUS,
                        vx = math.cos(ship.angle) * BULLET_SPEED,
                        vy = math.sin(ship.angle) * BULLET_SPEED,
                        lifetime = BULLET_LIFETIME,
                    }
                    ship.fire_cooldown = FIRE_COOLDOWN
                end

                -- Apply drag
                ship.vx = ship.vx * SHIP_DRAG
                ship.vy = ship.vy * SHIP_DRAG

                -- Move and wrap
                ship.x = ship.x + ship.vx * dt
                ship.y = ship.y + ship.vy * dt
                ship.x, ship.y = wrap_position(ship.x, ship.y)
            end
        end

        -- Update bullets
        local expired_bullets = {}
        for bullet_id, bullet in pairs(state.bullets) do
            bullet.x = bullet.x + bullet.vx * dt
            bullet.y = bullet.y + bullet.vy * dt
            bullet.x, bullet.y = wrap_position(bullet.x, bullet.y)
            bullet.lifetime = bullet.lifetime - dt

            if bullet.lifetime <= 0 then
                table.insert(expired_bullets, bullet_id)
            end
        end
        for _, id in ipairs(expired_bullets) do
            state.bullets[id] = nil
        end

        -- Update asteroids
        for _, asteroid in pairs(state.asteroids) do
            asteroid.x = asteroid.x + asteroid.vx * dt
            asteroid.y = asteroid.y + asteroid.vy * dt
            asteroid.x, asteroid.y = wrap_position(asteroid.x, asteroid.y)
        end

        -- Collision: bullets vs asteroids
        local bullets_hit = {}
        local asteroids_hit = {}
        local new_asteroids = {}

        for bullet_id, bullet in pairs(state.bullets) do
            if not bullets_hit[bullet_id] then
                for asteroid_id, asteroid in pairs(state.asteroids) do
                    if not asteroids_hit[asteroid_id] then
                        local radius = asteroid_radius(asteroid.size)
                        local dist_sq = distance_squared(bullet.x, bullet.y, asteroid.x, asteroid.y)
                        local hit_dist = BULLET_RADIUS + radius

                        if dist_sq < hit_dist * hit_dist then
                            bullets_hit[bullet_id] = true
                            asteroids_hit[asteroid_id] = true

                            -- Award points to shooter
                            local points = asteroid_points(asteroid.size)
                            for _, ship in pairs(state.ships) do
                                if ship.id == bullet.owner_id then
                                    ship.score = ship.score + points
                                    break
                                end
                            end

                            -- Split asteroid
                            if asteroid.size > 1 then
                                for i = 1, 2 do
                                    table.insert(new_asteroids, {
                                        size = asteroid.size - 1,
                                        x = asteroid.x,
                                        y = asteroid.y
                                    })
                                end
                            end

                            break
                        end
                    end
                end
            end
        end

        -- Remove hit bullets and asteroids
        for id in pairs(bullets_hit) do
            state.bullets[id] = nil
        end
        for id in pairs(asteroids_hit) do
            state.asteroids[id] = nil
        end

        -- Add split asteroids
        for _, ast in ipairs(new_asteroids) do
            spawn_asteroid(state, ast.size, ast.x, ast.y)
        end

        -- Respawn asteroids if too few
        local asteroid_count = 0
        for _ in pairs(state.asteroids) do asteroid_count = asteroid_count + 1 end
        while asteroid_count < INITIAL_ASTEROIDS do
            spawn_asteroid(state, 3)
            asteroid_count = asteroid_count + 1
        end

        -- Collision: ships vs asteroids
        for player_id, ship in pairs(state.ships) do
            if ship.alive then
                for _, asteroid in pairs(state.asteroids) do
                    local radius = asteroid_radius(asteroid.size)
                    local dist_sq = distance_squared(ship.x, ship.y, asteroid.x, asteroid.y)
                    local hit_dist = SHIP_RADIUS + radius

                    if dist_sq < hit_dist * hit_dist then
                        ship.alive = false
                        ship.respawn_timer = RESPAWN_TIME
                        kaos.logger_info(ship.name .. " was destroyed! (score: " .. ship.score .. ")")
                        break
                    end
                end
            end
        end

        -- Build broadcast-ready state (arrays for network serialization)
        local ships_array = {}
        for _, ship in pairs(state.ships) do
            table.insert(ships_array, {
                id = ship.id,
                name = ship.name,
                x = ship.x,
                y = ship.y,
                angle = ship.angle,
                score = ship.score,
                alive = ship.alive,
            })
        end

        local asteroids_array = {}
        for _, asteroid in pairs(state.asteroids) do
            table.insert(asteroids_array, {
                id = asteroid.id,
                x = asteroid.x,
                y = asteroid.y,
                size = asteroid.size,
            })
        end

        local bullets_array = {}
        for _, bullet in pairs(state.bullets) do
            table.insert(bullets_array, {
                x = bullet.x,
                y = bullet.y,
            })
        end

        -- Get leaderboard
        local leaderboard = {}
        local lb_records = kaos.leaderboard_list(LEADERBOARD_ID, 5) or {}
        for _, record in ipairs(lb_records) do
            table.insert(leaderboard, {
                name = record.username,
                score = record.score,
            })
        end

        state.broadcast = {
            tick = tick,
            ships = ships_array,
            asteroids = asteroids_array,
            bullets = bullets_array,
            leaderboard = leaderboard,
        }

        return state
    end,

    -- Match terminates
    match_terminate = function(ctx, state)
        kaos.logger_info("Kaos Asteroids match terminated")
    end
})

kaos.logger_info("Kaos Asteroids game module loaded")
