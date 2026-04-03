/**
 * Input Validation Functions
 * Validates node, edge, and map data
 */

import { INPUT_VALIDATION, MAP_BOUNDS } from './constants';

export class ValidationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ValidationError';
  }
}

/**
 * Validates node position is within map bounds
 */
export function validateNodePosition(x: number, y: number): void {
  if (!isFinite(x) || !isFinite(y)) {
    throw new ValidationError('Position coordinates must be valid numbers');
  }
  
  if (x < MAP_BOUNDS.MIN_X || x > MAP_BOUNDS.MAX_X) {
    throw new ValidationError(`Node X position must be between ${MAP_BOUNDS.MIN_X} and ${MAP_BOUNDS.MAX_X}`);
  }
  
  if (y < MAP_BOUNDS.MIN_Y || y > MAP_BOUNDS.MAX_Y) {
    throw new ValidationError(`Node Y position must be between ${MAP_BOUNDS.MIN_Y} and ${MAP_BOUNDS.MAX_Y}`);
  }
}

/**
 * Validates node name
 */
export function validateNodeName(name: string | null | undefined): void {
  if (!name || typeof name !== 'string') {
    throw new ValidationError('Node name is required');
  }
  
  const trimmed = name.trim();
  if (trimmed.length < INPUT_VALIDATION.NODE_NAME_MIN) {
    throw new ValidationError('Node name cannot be empty');
  }
  
  if (trimmed.length > INPUT_VALIDATION.NODE_NAME_MAX) {
    throw new ValidationError(`Node name cannot exceed ${INPUT_VALIDATION.NODE_NAME_MAX} characters`);
  }
}

/**
 * Validates node kind
 */
export function validateNodeKind(kind: string): void {
  const validKinds = ['Intersection', 'Habitation', 'Workplace'];
  if (!validKinds.includes(kind)) {
    throw new ValidationError(`Node kind must be one of: ${validKinds.join(', ')}`);
  }
}

/**
 * Validates entire node creation data
 */
export function validateNodeCreation(x: number, y: number, name: string, kind: string): void {
  validateNodePosition(x, y);
  validateNodeName(name);
  validateNodeKind(kind);
}

/**
 * Validates lane count
 */
export function validateLaneCount(count: number): void {
  if (!Number.isInteger(count)) {
    throw new ValidationError('Lane count must be an integer');
  }
  
  if (count < INPUT_VALIDATION.LANE_COUNT_MIN || count > INPUT_VALIDATION.LANE_COUNT_MAX) {
    throw new ValidationError(
      `Lane count must be between ${INPUT_VALIDATION.LANE_COUNT_MIN} and ${INPUT_VALIDATION.LANE_COUNT_MAX}`
    );
  }
}

/**
 * Validates speed limit
 */
export function validateSpeedLimit(limit: number): void {
  if (!isFinite(limit)) {
    throw new ValidationError('Speed limit must be a valid number');
  }
  
  if (limit < INPUT_VALIDATION.SPEED_LIMIT_MIN || limit > INPUT_VALIDATION.SPEED_LIMIT_MAX) {
    throw new ValidationError(
      `Speed limit must be between ${INPUT_VALIDATION.SPEED_LIMIT_MIN} and ${INPUT_VALIDATION.SPEED_LIMIT_MAX} km/h`
    );
  }
}

/**
 * Validates entire edge (road) data
 */
export function validateEdgeUpdate(
  lane_count: number,
  speed_limit: number,
  is_blocked: boolean,
  can_overtake: boolean
): void {
  validateLaneCount(lane_count);
  validateSpeedLimit(speed_limit);
  
  if (typeof is_blocked !== 'boolean') {
    throw new ValidationError('is_blocked must be a boolean');
  }
  
  if (typeof can_overtake !== 'boolean') {
    throw new ValidationError('can_overtake must be a boolean');
  }
}

/**
 * Validates nodes are different (for road creation)
 */
export function validateDifferentNodes(fromId: number, toId: number): void {
  if (fromId === toId) {
    throw new ValidationError('Cannot create road between the same node');
  }
}
