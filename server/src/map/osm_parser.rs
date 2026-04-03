use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use osmpbf::{Element, ElementReader};

use crate::map::intersection::IntersectionKind;
use crate::map::model::Map;

// ── Error type ──────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum OsmParseError {
    Io(std::io::Error),
    Osm(osmpbf::Error),
    NoHighways,
}

impl fmt::Display for OsmParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OsmParseError::Io(e) => write!(f, "IO error: {}", e),
            OsmParseError::Osm(e) => write!(f, "OSM parse error: {}", e),
            OsmParseError::NoHighways => write!(f, "No highway ways found in the PBF file"),
        }
    }
}

impl From<std::io::Error> for OsmParseError {
    fn from(e: std::io::Error) -> Self {
        OsmParseError::Io(e)
    }
}

impl From<osmpbf::Error> for OsmParseError {
    fn from(e: osmpbf::Error) -> Self {
        OsmParseError::Osm(e)
    }
}

// ── Intermediate types ──────────────────────────────────────────────────

/// A highway way collected during pass 1.
struct HighwayWay {
    node_refs: Vec<i64>,
    highway_type: String,
    speed_limit: Option<f32>, // m/s
    lane_count: u8,
    oneway: bool,
}

/// A node's position (lat/lon in degrees).
#[derive(Clone, Copy)]
struct NodeCoord {
    lat: f64,
    lon: f64,
}

// ── Main entry point ────────────────────────────────────────────────────

/// Parse an `.osm.pbf` file and build a [`Map`].
///
/// # Example
/// ```ignore
/// use server::map::osm_parser::parse_osm_pbf;
/// let map = parse_osm_pbf("path/to/region.osm.pbf").unwrap();
/// println!("Intersections: {}", map.graph.node_count());
/// println!("Roads: {}", map.graph.edge_count());
/// ```
pub fn parse_osm_pbf<P: AsRef<Path>>(path: P) -> Result<Map, OsmParseError> {
    // ── Pass 1: collect highway ways & count node references ────────
    let (ways, mut node_ref_count) = collect_highway_data(path.as_ref())?;
    if ways.is_empty() {
        return Err(OsmParseError::NoHighways);
    }

    // Mark endpoints of every way as intersection nodes (ref count ≥ 2).
    for way in &ways {
        if let (Some(&first), Some(&last)) = (way.node_refs.first(), way.node_refs.last()) {
            *node_ref_count.entry(first).or_insert(0) += 1;
            *node_ref_count.entry(last).or_insert(0) += 1;
        }
    }

    // Collect the set of node IDs we need coordinates for.
    let needed_nodes: std::collections::HashSet<i64> = node_ref_count.keys().copied().collect();

    // ── Pass 2: fetch node coordinates ──────────────────────────────
    let node_coords = collect_node_coords(path.as_ref(), &needed_nodes)?;

    // ── Build the Map ───────────────────────────────────────────────
    build_map(&ways, &node_ref_count, &node_coords)
}

// ── Pass 1 ──────────────────────────────────────────────────────────────

/// Accepted highway types (car-accessible roads).
const ACCEPTED_HIGHWAY_TYPES: &[&str] = &[
    "motorway",
    "trunk",
    "primary",
    "secondary",
    "tertiary",
    "residential",
    "unclassified",
    "living_street",
    "service",
    "motorway_link",
    "trunk_link",
    "primary_link",
    "secondary_link",
    "tertiary_link",
];

fn collect_highway_data(
    path: &Path,
) -> Result<(Vec<HighwayWay>, HashMap<i64, u32>), OsmParseError> {
    let reader = ElementReader::from_path(path)?;
    let mut ways: Vec<HighwayWay> = Vec::new();
    let mut node_ref_count: HashMap<i64, u32> = HashMap::new();

    reader.for_each(|element| {
        if let Element::Way(way) = element {
            let tags: Vec<(&str, &str)> = way.tags().map(|(k, v)| (k, v)).collect();

            let highway_type = tags
                .iter()
                .find(|(k, _)| *k == "highway")
                .map(|(_, v)| *v);

            if let Some(hw_type) = highway_type {
                if !ACCEPTED_HIGHWAY_TYPES.contains(&hw_type) {
                    return;
                }

                let node_refs: Vec<i64> = way.refs().collect();
                if node_refs.len() < 2 {
                    return;
                }

                // Count how many ways reference each node.
                for &node_id in &node_refs {
                    *node_ref_count.entry(node_id).or_insert(0) += 1;
                }

                let speed_limit = tags
                    .iter()
                    .find(|(k, _)| *k == "maxspeed")
                    .and_then(|(_, v)| parse_speed_limit(v));

                let lane_count = tags
                    .iter()
                    .find(|(k, _)| *k == "lanes")
                    .and_then(|(_, v)| v.parse::<u8>().ok())
                    .unwrap_or(1);

                let oneway = tags.iter().any(|(k, v)| {
                    *k == "oneway" && (*v == "yes" || *v == "1" || *v == "true")
                }) || hw_type == "motorway"
                    || hw_type == "motorway_link";

                ways.push(HighwayWay {
                    node_refs,
                    highway_type: hw_type.to_string(),
                    speed_limit,
                    lane_count,
                    oneway,
                });
            }
        }
    })?;

    Ok((ways, node_ref_count))
}

// ── Pass 2: node coordinates ────────────────────────────────────────────

fn collect_node_coords(
    path: &Path,
    needed: &std::collections::HashSet<i64>,
) -> Result<HashMap<i64, NodeCoord>, OsmParseError> {
    let reader = ElementReader::from_path(path)?;
    let mut coords: HashMap<i64, NodeCoord> = HashMap::with_capacity(needed.len());

    reader.for_each(|element| {
        match element {
            Element::Node(node) => {
                if needed.contains(&node.id()) {
                    coords.insert(
                        node.id(),
                        NodeCoord {
                            lat: node.lat(),
                            lon: node.lon(),
                        },
                    );
                }
            }
            Element::DenseNode(node) => {
                if needed.contains(&node.id) {
                    coords.insert(
                        node.id,
                        NodeCoord {
                            lat: node.lat(),
                            lon: node.lon(),
                        },
                    );
                }
            }
            _ => {}
        }
    })?;

    Ok(coords)
}

// ── Map builder ─────────────────────────────────────────────────────────

fn build_map(
    ways: &[HighwayWay],
    node_ref_count: &HashMap<i64, u32>,
    node_coords: &HashMap<i64, NodeCoord>,
) -> Result<Map, OsmParseError> {
    let mut map = Map::new();

    // Compute a geographic center for the equirectangular projection.
    let (center_lat, center_lon) = {
        let mut lat_sum = 0.0_f64;
        let mut lon_sum = 0.0_f64;
        let mut count = 0u64;
        for coord in node_coords.values() {
            lat_sum += coord.lat;
            lon_sum += coord.lon;
            count += 1;
        }
        if count == 0 {
            return Err(OsmParseError::NoHighways);
        }
        (lat_sum / count as f64, lon_sum / count as f64)
    };

    // Maps an OSM node ID → Map intersection id (u32).
    let mut node_index_map: HashMap<i64, u32> = HashMap::new();

    for way in ways {
        let speed = way
            .speed_limit
            .unwrap_or_else(|| default_speed_limit(&way.highway_type));

        // Walk the way's node list; split at any node referenced by ≥ 2
        // ways (i.e. intersection nodes).
        let mut segment_start_idx: usize = 0;

        for i in 1..way.node_refs.len() {
            let node_id = way.node_refs[i];
            let is_intersection = node_ref_count.get(&node_id).copied().unwrap_or(0) >= 2;
            let is_last = i == way.node_refs.len() - 1;

            if !is_intersection && !is_last {
                continue;
            }

            // Build segment [segment_start_idx .. i]
            let segment_nodes = &way.node_refs[segment_start_idx..=i];
            let length = compute_segment_length(segment_nodes, node_coords);

            if length < 0.01 {
                segment_start_idx = i;
                continue;
            }

            let start_osm_id = way.node_refs[segment_start_idx];
            let end_osm_id = way.node_refs[i];

            let coord_start = match node_coords.get(&start_osm_id) {
                Some(c) => c,
                None => {
                    segment_start_idx = i;
                    continue;
                }
            };
            let coord_end = match node_coords.get(&end_osm_id) {
                Some(c) => c,
                None => {
                    segment_start_idx = i;
                    continue;
                }
            };

            let from_id = *node_index_map.entry(start_osm_id).or_insert_with(|| {
                let (x, y) = project_coords(coord_start.lat, coord_start.lon, center_lat, center_lon);
                map.add_intersection(IntersectionKind::Habitation, x, y)
            });

            let to_id = *node_index_map.entry(end_osm_id).or_insert_with(|| {
                let (x, y) = project_coords(coord_end.lat, coord_end.lon, center_lat, center_lon);
                map.add_intersection(IntersectionKind::Habitation, x, y)
            });

            if way.oneway {
                map.add_road(from_id, to_id, way.lane_count, speed, length);
            } else {
                map.add_two_way_road(from_id, to_id, way.lane_count, speed, length);
            }

            segment_start_idx = i;
        }
    }

    Ok(map)
}

// ── Helper functions ────────────────────────────────────────────────────

/// Compute the total length (in meters) of a segment defined by a slice of
/// OSM node IDs, using the haversine formula.
fn compute_segment_length(
    node_ids: &[i64],
    coords: &HashMap<i64, NodeCoord>,
) -> f32 {
    let mut total = 0.0_f64;
    for window in node_ids.windows(2) {
        if let (Some(a), Some(b)) = (coords.get(&window[0]), coords.get(&window[1])) {
            total += haversine_distance(a.lat, a.lon, b.lat, b.lon);
        }
    }
    total as f32
}

/// Haversine distance between two lat/lon points, returns meters.
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const R: f64 = 6_371_000.0; // Earth radius in meters

    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();

    let a = (d_lat / 2.0).sin().powi(2)
        + lat1_r.cos() * lat2_r.cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    R * c
}

/// Equirectangular projection: convert (lat, lon) → (x, y) in meters,
/// relative to (center_lat, center_lon). Output is centered around (0, 0).
/// The y-axis is negated so that north points upward on screen (canvas/PixiJS
/// y-axis increases downward).
fn project_coords(lat: f64, lon: f64, center_lat: f64, center_lon: f64) -> (f32, f32) {
    const R: f64 = 6_371_000.0;
    let x = (lon - center_lon).to_radians() * center_lat.to_radians().cos() * R;
    let y = -(lat - center_lat).to_radians() * R;
    (x as f32, y as f32)
}

/// Parse a speed tag string into m/s.
///
/// Handles formats like:
///  - `"50"` → 50 km/h → 13.89 m/s
///  - `"30 mph"` → 30 mph → 13.41 m/s
///  - `"walk"` → 5 km/h
fn parse_speed_limit(tag: &str) -> Option<f32> {
    let tag = tag.trim();

    if tag.eq_ignore_ascii_case("walk") {
        return Some(5.0 / 3.6);
    }

    if let Some(mph_str) = tag.strip_suffix("mph") {
        let mph: f32 = mph_str.trim().parse().ok()?;
        return Some(mph * 1.60934 / 3.6);
    }

    // Default: km/h
    let kmh_str = tag.strip_suffix("km/h").unwrap_or(tag);
    let kmh: f32 = kmh_str.trim().parse().ok()?;
    Some(kmh / 3.6)
}

/// Default speed limit in m/s for a given highway type.
fn default_speed_limit(highway_type: &str) -> f32 {
    let kmh: f32 = match highway_type {
        "motorway" | "motorway_link" => 130.0,
        "trunk" | "trunk_link" => 110.0,
        "primary" | "primary_link" => 80.0,
        "secondary" | "secondary_link" => 70.0,
        "tertiary" | "tertiary_link" => 50.0,
        "residential" => 30.0,
        "living_street" => 20.0,
        "service" => 20.0,
        "unclassified" => 50.0,
        _ => 50.0,
    };
    kmh / 3.6
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_speed_limit() {
        // km/h formats
        assert!((parse_speed_limit("50").unwrap() - 50.0 / 3.6).abs() < 0.01);
        assert!((parse_speed_limit("30 km/h").unwrap() - 30.0 / 3.6).abs() < 0.01);

        // mph
        let mph30 = 30.0 * 1.60934 / 3.6;
        assert!((parse_speed_limit("30 mph").unwrap() - mph30).abs() < 0.01);
        assert!((parse_speed_limit("30mph").unwrap() - mph30).abs() < 0.01);

        // walk
        assert!((parse_speed_limit("walk").unwrap() - 5.0 / 3.6).abs() < 0.01);

        // invalid
        assert!(parse_speed_limit("none").is_none());
    }

    #[test]
    fn test_haversine_distance() {
        // Paris (48.8566, 2.3522) → Lyon (45.7640, 4.8357) ≈ 392 km
        let d = haversine_distance(48.8566, 2.3522, 45.7640, 4.8357);
        assert!((d - 392_000.0).abs() < 5_000.0, "Got {} m", d);
    }

    #[test]
    fn test_project_coords() {
        // Projecting the center itself should give (0, 0).
        let (x, y) = project_coords(45.0, 2.0, 45.0, 2.0);
        assert!(x.abs() < 1.0);
        assert!(y.abs() < 1.0);

        // Non-center coords should give non-zero projections.
        let (x2, y2) = project_coords(48.8566, 2.3522, 48.0, 2.0);
        assert!(x2 != 0.0);
        assert!(y2 != 0.0);
    }

    #[test]
    fn test_default_speed_limit() {
        assert!((default_speed_limit("motorway") - 130.0 / 3.6).abs() < 0.01);
        assert!((default_speed_limit("residential") - 30.0 / 3.6).abs() < 0.01);
        assert!((default_speed_limit("unknown_type") - 50.0 / 3.6).abs() < 0.01);
    }
}
