/**
 * Map Editor Constants
 * Centralizes all magic numbers and configuration values
 */

export const MAP_CONFIG = {
  // Node hit detection
  NODE_RADIUS: 10,              // Visual radius of node circle
  NODE_GLOW_RADIUS: 6,          // Selection/source glow ring
  NODE_HIT_RADIUS: 16,          // Hit test radius (10 + 6)
  
  // Node colors
  NODE_COLORS: {
    Intersection: 0xaaaaaa,     // Gray
    Habitation: 0x3366ff,       // Blue
    Workplace: 0xff3333,        // Red
  },
  
  // Road hit detection
  ROAD_WIDTH: 15,               // Visual width of road
  ROAD_HIT_RADIUS: 7.5,         // Hit test radius (width/2)
  
  // Stage/rendering
  BACKGROUND_COLOR: 0xC1D9B7,   // Light green background
  
  // Default values
  DEFAULT_LANE_COUNT: 1,
  DEFAULT_SPEED_LIMIT: 40,
  
  // Rubber-band line
  RUBBER_BAND_COLOR: 0xffff00,
  RUBBER_BAND_WIDTH: 2,
  RUBBER_BAND_ALPHA: 0.8,
};

export const MAP_BOUNDS = {
  MIN_X: 0,
  MAX_X: 10000,
  MIN_Y: 0,
  MAX_Y: 10000,
};

export const INPUT_VALIDATION = {
  NODE_NAME_MIN: 1,
  NODE_NAME_MAX: 50,
  LANE_COUNT_MIN: 1,
  LANE_COUNT_MAX: 6,
  SPEED_LIMIT_MIN: 0,
  SPEED_LIMIT_MAX: 300,
};

export const TOAST_DEFAULTS = {
  SUCCESS_DURATION: 3000,
  ERROR_DURATION: 4000,
  WARNING_DURATION: 3500,
  INFO_DURATION: 2500,
};

export const KEYBOARD_SHORTCUTS = {
  EDIT_MODE: ['e', 'E'],
  SELECT_TOOL: ['v', 'V'],
  ADD_NODE_TOOL: ['n', 'N'],
  ADD_ROAD_TOOL: ['r', 'R'],
  DELETE: ['Delete', 'Backspace'],
  DESELECT: ['Escape'],
};
