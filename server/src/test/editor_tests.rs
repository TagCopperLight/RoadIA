use crate::map::editor::{
    add_node, add_road, add_roundabout, delete_node, delete_road, move_node, update_node,
    update_road,
};
use crate::map::intersection::{build_intersections, IntersectionKind};
use crate::map::model::Map;
use crate::map::roundabout::{finalize_roundabout_links, RoundaboutHandle};
use crate::map::road::LinkType;
use crate::simulation::config::MAX_SPEED;

fn make_two_node_map() -> (Map, u32, u32) {
    let mut map = Map::new();
    let a = add_node(&mut map, 0.0, 0.0, IntersectionKind::Habitation);
    let b = add_node(&mut map, 300.0, 400.0, IntersectionKind::Workplace);
    (map, a, b)
}

// ---- add_node ----

#[test]
fn add_node_returns_incrementing_ids() {
    let mut map = Map::new();
    let id0 = add_node(&mut map, 0.0, 0.0, IntersectionKind::Habitation);
    let id1 = add_node(&mut map, 10.0, 0.0, IntersectionKind::Workplace);
    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
}

#[test]
fn add_node_is_findable() {
    let mut map = Map::new();
    let id = add_node(&mut map, 5.0, 10.0, IntersectionKind::Intersection);
    assert!(map.find_node(id).is_some());
}

#[test]
fn add_node_stores_correct_coordinates() {
    let mut map = Map::new();
    let id = add_node(&mut map, 12.5, 99.0, IntersectionKind::Habitation);
    let ni = map.find_node(id).unwrap();
    let node = &map.graph[ni];
    assert!((node.center_coordinates.x - 12.5).abs() < 1e-4);
    assert!((node.center_coordinates.y - 99.0).abs() < 1e-4);
}

// ---- delete_node ----

#[test]
fn delete_node_existing_succeeds() {
    let (mut map, a, _b) = make_two_node_map();
    assert!(delete_node(&mut map, a).is_ok());
    assert!(map.find_node(a).is_none());
}

#[test]
fn delete_node_missing_returns_err() {
    let mut map = Map::new();
    let err = delete_node(&mut map, 9999);
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("not found"));
}

#[test]
fn delete_node_updates_swapped_index() {
    // Add A, B, C. Delete B. petgraph swaps C into B's slot.
    // C's node_index_map entry must be updated so find_node(C_id) still works.
    let mut map = Map::new();
    let a_id = add_node(&mut map, 0.0, 0.0, IntersectionKind::Habitation);
    let b_id = add_node(&mut map, 10.0, 0.0, IntersectionKind::Intersection);
    let c_id = add_node(&mut map, 20.0, 0.0, IntersectionKind::Workplace);

    delete_node(&mut map, b_id).unwrap();

    // A and C must still be findable
    assert!(map.find_node(a_id).is_some(), "A should still exist");
    assert!(map.find_node(c_id).is_some(), "C should still exist after swap-fix");
    // B must be gone
    assert!(map.find_node(b_id).is_none(), "B should be removed");
}

// ---- move_node ----

#[test]
fn move_node_updates_coordinates() {
    let (mut map, a, _b) = make_two_node_map();
    move_node(&mut map, a, 99.0, 77.0).unwrap();
    let ni = map.find_node(a).unwrap();
    let node = &map.graph[ni];
    assert!((node.center_coordinates.x - 99.0).abs() < 1e-4);
    assert!((node.center_coordinates.y - 77.0).abs() < 1e-4);
}

#[test]
fn move_node_recalculates_connected_road_length() {
    let (mut map, a, b) = make_two_node_map();
    // Initial length: dist((0,0),(300,400)) = 500.0
    add_road(&mut map, a, b, 1, 30.0).unwrap();

    // Move b to (0, 100) → center-to-center = 100, boundary-to-boundary = 98 (two radii of 1)
    move_node(&mut map, b, 0.0, 100.0).unwrap();

    let ni_a = map.find_node(a).unwrap();
    let ni_b = map.find_node(b).unwrap();
    let edge = map.graph.find_edge(ni_a, ni_b).unwrap();
    let new_length = map.graph[edge].length;
    assert!((new_length - 98.0).abs() < 0.5, "expected ~98, got {new_length}");
}

#[test]
fn move_node_missing_returns_err() {
    let mut map = Map::new();
    assert!(move_node(&mut map, 9999, 0.0, 0.0).is_err());
}

// ---- update_node ----

#[test]
fn update_node_changes_kind() {
    let mut map = Map::new();
    let id = add_node(&mut map, 0.0, 0.0, IntersectionKind::Habitation);
    update_node(&mut map, id, IntersectionKind::Workplace).unwrap();
    let ni = map.find_node(id).unwrap();
    assert!(matches!(map.graph[ni].kind, IntersectionKind::Workplace));
}

#[test]
fn update_node_missing_returns_err() {
    let mut map = Map::new();
    assert!(update_node(&mut map, 9999, IntersectionKind::Intersection).is_err());
}

// ---- add_road ----

#[test]
fn add_road_calculates_euclidean_length() {
    let (mut map, a, b) = make_two_node_map();
    // a=(0,0), b=(300,400) → center-to-center=500, boundary-to-boundary=498 (two radii of 1)
    let road_id = add_road(&mut map, a, b, 1, 30.0).unwrap();
    let edge = map.find_edge(road_id).unwrap();
    let length = map.graph[edge].length;
    assert!((length - 498.0).abs() < 0.5, "expected 498, got {length}");
}

#[test]
fn add_road_sets_speed_limit() {
    let (mut map, a, b) = make_two_node_map();
    let road_id = add_road(&mut map, a, b, 1, 25.0).unwrap();
    let edge = map.find_edge(road_id).unwrap();
    assert!((map.graph[edge].speed_limit - 25.0).abs() < 1e-4);
}

#[test]
fn add_road_duplicate_returns_err() {
    let (mut map, a, b) = make_two_node_map();
    add_road(&mut map, a, b, 1, 30.0).unwrap();
    let result = add_road(&mut map, a, b, 1, 30.0);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));
}

#[test]
fn add_road_missing_from_node_returns_err() {
    let (mut map, _a, b) = make_two_node_map();
    assert!(add_road(&mut map, 9999, b, 1, 30.0).is_err());
}

#[test]
fn add_road_missing_to_node_returns_err() {
    let (mut map, a, _b) = make_two_node_map();
    assert!(add_road(&mut map, a, 9999, 1, 30.0).is_err());
}

// ---- delete_road ----

#[test]
fn delete_road_removes_edge() {
    let (mut map, a, b) = make_two_node_map();
    let road_id = add_road(&mut map, a, b, 1, 30.0).unwrap();
    assert!(map.find_edge(road_id).is_some());
    delete_road(&mut map, road_id).unwrap();
    assert!(map.find_edge(road_id).is_none());
}

#[test]
fn delete_road_missing_returns_err() {
    let mut map = Map::new();
    assert!(delete_road(&mut map, 9999).is_err());
}

// ---- update_road ----

#[test]
fn update_road_changes_speed_limit() {
    let (mut map, a, b) = make_two_node_map();
    let road_id = add_road(&mut map, a, b, 1, 20.0).unwrap();
    update_road(&mut map, road_id, 35.0).unwrap();
    let edge = map.find_edge(road_id).unwrap();
    assert!((map.graph[edge].speed_limit - 35.0).abs() < 1e-4);
}

#[test]
fn update_road_clamps_to_max_speed() {
    let (mut map, a, b) = make_two_node_map();
    let road_id = add_road(&mut map, a, b, 1, 20.0).unwrap();
    update_road(&mut map, road_id, 9999.0).unwrap();
    let edge = map.find_edge(road_id).unwrap();
    assert_eq!(map.graph[edge].speed_limit, MAX_SPEED);
}

#[test]
fn update_road_missing_returns_err() {
    let mut map = Map::new();
    assert!(update_road(&mut map, 9999, 30.0).is_err());
}

// ---- add_roundabout ----

fn make_roundabout_map() -> (Map, RoundaboutHandle) {
    let mut map = Map::new();
    let north = map.add_intersection(IntersectionKind::Habitation, 500.0, 0.0);
    let east = map.add_intersection(IntersectionKind::Workplace, 1000.0, 500.0);
    let south = map.add_intersection(IntersectionKind::Habitation, 500.0, 1000.0);
    let west = map.add_intersection(IntersectionKind::Workplace, 0.0, 500.0);

    let handle = add_roundabout(&mut map, 500.0, 500.0, 40.0, 4, 30.0, 1);

    map.add_two_way_road(north, handle.ring_node_ids[0], 1, 30.0, 460.0);
    map.add_two_way_road(east, handle.ring_node_ids[1], 1, 30.0, 460.0);
    map.add_two_way_road(south, handle.ring_node_ids[2], 1, 30.0, 460.0);
    map.add_two_way_road(west, handle.ring_node_ids[3], 1, 30.0, 460.0);

    build_intersections(&mut map);
    finalize_roundabout_links(&mut map, &handle);

    (map, handle)
}

#[test]
fn add_roundabout_ring_node_count() {
    let (_, handle) = make_roundabout_map();
    assert_eq!(handle.ring_node_ids.len(), 4);
}

#[test]
fn add_roundabout_ring_road_count() {
    let (_, handle) = make_roundabout_map();
    assert_eq!(handle.ring_road_ids.len(), 4);
}

#[test]
fn add_roundabout_ring_nodes_at_correct_radius() {
    let (map, handle) = make_roundabout_map();
    for &id in &handle.ring_node_ids {
        let idx = map.find_node(id).unwrap();
        let n = &map.graph[idx];
        let dx = n.center_coordinates.x - 500.0;
        let dy = n.center_coordinates.y - 500.0;
        let r = (dx * dx + dy * dy).sqrt();
        assert!((r - 40.0).abs() < 1e-3, "node {id} is at radius {r}, expected 40.0");
    }
}

#[test]
fn add_roundabout_ring_nodes_equidistant() {
    let (map, handle) = make_roundabout_map();
    let n = handle.ring_node_ids.len();
    let chords: Vec<f32> = (0..n)
        .map(|i| {
            let a = map.find_node(handle.ring_node_ids[i]).unwrap();
            let b = map.find_node(handle.ring_node_ids[(i + 1) % n]).unwrap();
            let ax = map.graph[a].center_coordinates.x;
            let ay = map.graph[a].center_coordinates.y;
            let bx = map.graph[b].center_coordinates.x;
            let by = map.graph[b].center_coordinates.y;
            ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt()
        })
        .collect();
    let first = chords[0];
    for (i, &c) in chords.iter().enumerate() {
        assert!((c - first).abs() < 1e-2, "chord {i} length {c} != {first}");
    }
}

#[test]
fn add_roundabout_ring_edges_are_one_way() {
    let (map, handle) = make_roundabout_map();
    let n = handle.ring_node_ids.len();
    for i in 0..n {
        let from = map.find_node(handle.ring_node_ids[(i + 1) % n]).unwrap();
        let to = map.find_node(handle.ring_node_ids[i]).unwrap();
        assert!(map.graph.find_edge(from, to).is_some());
        // The reverse edge must not be a ring road.
        if let Some(rev) = map.graph.find_edge(to, from) {
            assert!(
                !handle.ring_road_ids.contains(&map.graph[rev].id),
                "ring segment {i} has an unexpected reverse ring edge"
            );
        }
    }
}

#[test]
fn add_roundabout_entry_links_are_yield() {
    use petgraph::visit::EdgeRef;
    use petgraph::Direction;
    use std::collections::HashSet;

    let (map, handle) = make_roundabout_map();
    let ring_set: HashSet<u32> = handle.ring_node_ids.iter().copied().collect();

    for &ring_id in &handle.ring_node_ids {
        let ring_idx = map.find_node(ring_id).unwrap();
        for e in map.graph.edges_directed(ring_idx, Direction::Incoming) {
            let (src, _) = map.graph.edge_endpoints(e.id()).unwrap();
            if ring_set.contains(&map.graph[src].id) {
                continue;
            }
            for lane in &map.graph[e.id()].lanes {
                for link in &lane.links {
                    assert_eq!(link.link_type, LinkType::Yield, "entry link {} should be Yield", link.id);
                }
            }
        }
    }
}

#[test]
fn add_roundabout_ring_segment_links_are_priority() {
    let (map, handle) = make_roundabout_map();
    for &road_id in &handle.ring_road_ids {
        let edge = map.find_edge(road_id).unwrap();
        for lane in &map.graph[edge].lanes {
            for link in &lane.links {
                assert_eq!(link.link_type, LinkType::Priority, "ring link {} should be Priority", link.id);
            }
        }
    }
}
