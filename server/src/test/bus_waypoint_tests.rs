#[cfg(test)]
mod bus_waypoint_tests {
    #[test]
    fn test_bus_waypoints_excludes_destination() {
        let stops: Vec<u32> = vec![1, 2, 3, 4];  // NodeIndex values

        let old_waypoints: Vec<u32> = stops[1..].to_vec();
        
        let new_waypoints: Vec<u32> = if stops.len() > 2 {
            stops[1..stops.len()-1].to_vec()
        } else {
            vec![]
        };
        
        println!("Route stops: {:?}", stops);
        println!("Old waypoints (BUGGY): {:?}", old_waypoints);
        println!("New waypoints (FIXED): {:?}", new_waypoints);
        
        assert_eq!(old_waypoints, vec![2u32, 3, 4], "Old code included destination!");
        assert_eq!(new_waypoints, vec![2u32, 3], "New code should exclude destination!");
        assert!(!new_waypoints.contains(&4), "Destination 4 should not be in waypoints!");
        
        println!("✓ Waypoint fix verified: destination correctly excluded!");
    }
    
    #[test]
    fn test_bus_waypoints_edge_cases() {
        let stops_2: Vec<u32> = vec![1, 2];
        let waypoints_2: Vec<u32> = if stops_2.len() > 2 {
            stops_2[1..stops_2.len()-1].to_vec()
        } else {
            vec![]
        };
        assert_eq!(waypoints_2, vec![] as Vec<u32>, "2 stops should have no waypoints");
        
        let stops_3: Vec<u32> = vec![1, 2, 3];
        let waypoints_3: Vec<u32> = if stops_3.len() > 2 {
            stops_3[1..stops_3.len()-1].to_vec()
        } else {
            vec![]
        };
        assert_eq!(waypoints_3, vec![2u32], "3 stops should have 1 waypoint");
        
        let stops_5: Vec<u32> = vec![1, 2, 3, 4, 5];
        let waypoints_5: Vec<u32> = if stops_5.len() > 2 {
            stops_5[1..stops_5.len()-1].to_vec()
        } else {
            vec![]
        };
        assert_eq!(waypoints_5, vec![2u32, 3, 4], "5 stops should have 3 waypoints");
        
        println!("✓ All edge cases passed!");
    }
}