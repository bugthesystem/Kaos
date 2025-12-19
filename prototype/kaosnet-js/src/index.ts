/**
 * KaosNet SDK - Official JavaScript/TypeScript SDK for KaosNet game server
 *
 * @example
 * ```typescript
 * import { KaosClient } from 'kaosnet-js';
 *
 * // Create client
 * const client = new KaosClient('localhost', 7350);
 *
 * // Authenticate
 * const session = await client.authenticateDevice('unique-device-id');
 *
 * // Connect to real-time socket
 * const socket = client.createSocket();
 * await socket.connect(session);
 *
 * // Join matchmaking
 * const ticket = await socket.addMatchmaker({
 *   query: '+mode:ranked +region:us',
 *   minCount: 2,
 *   maxCount: 4,
 *   stringProperties: { mode: 'ranked', region: 'us' },
 *   numericProperties: { skill: 1500 },
 * });
 *
 * // Handle match found
 * socket.onMatchmakerMatched = (match) => {
 *   console.log('Match found!', match.matchId);
 *   socket.joinMatch(match.matchId);
 * };
 *
 * // Send/receive game state
 * socket.onMatchState = (state) => {
 *   console.log('Game state:', state.data);
 * };
 *
 * socket.sendMatchState(1, { x: 100, y: 200 });
 * ```
 *
 * @packageDocumentation
 */

export { KaosClient } from './client';
export { KaosSocket, OpCode } from './socket';
export * from './types';

// Re-export for convenience
export { KaosClient as Client } from './client';
export { KaosSocket as Socket } from './socket';
