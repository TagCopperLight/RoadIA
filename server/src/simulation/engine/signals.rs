//! Traffic light runtime: phase timers and the resulting state of every signalised link.

use std::collections::HashMap;

use crate::map::model::Map;
use crate::map::road::LinkType;
use crate::simulation::vehicle::LaneId;

use super::network::Network;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Signal {
    /// The link is not controlled by a traffic light.
    Uncontrolled,
    Green,
    Yellow,
    Red,
}

#[derive(Clone)]
struct ControllerState {
    phase_index: usize,
    time_in_phase: f32,
}

#[derive(Clone, Default)]
pub(crate) struct Signals {
    controllers: HashMap<u32, ControllerState>,
    link_signals: Vec<Signal>,
    /// Roads with a green (or yellow) light, per controller. Shown by the client.
    pub green_road_ids: HashMap<u32, Vec<u32>>,
}

impl Signals {
    /// Builds the signal state for `map`, keeping the phase timers of `previous`
    /// controllers that still exist.
    pub fn build(map: &Map, network: &Network, previous: &Signals) -> Self {
        let controllers = map
            .traffic_lights
            .iter()
            .map(|(&id, controller)| {
                let state = previous
                    .controllers
                    .get(&id)
                    .filter(|state| state.phase_index < controller.phases.len())
                    .cloned()
                    .unwrap_or(ControllerState { phase_index: 0, time_in_phase: 0.0 });
                (id, state)
            })
            .collect();

        let mut signals = Signals {
            controllers,
            link_signals: Vec::new(),
            green_road_ids: HashMap::new(),
        };
        signals.refresh(map, network);
        signals
    }

    pub fn of(&self, link_id: u32) -> Signal {
        self.link_signals
            .get(link_id as usize)
            .copied()
            .unwrap_or(Signal::Uncontrolled)
    }

    pub fn advance(&mut self, map: &Map, network: &Network, dt: f32) {
        let mut changed = false;
        for (id, state) in &mut self.controllers {
            let Some(controller) = map.traffic_lights.get(id) else {
                continue;
            };
            let Some(phase) = controller.phases.get(state.phase_index) else {
                continue;
            };

            let was_green = state.time_in_phase < phase.green_duration;
            state.time_in_phase += dt;
            let cycle = phase.green_duration + phase.yellow_duration;
            if state.time_in_phase >= cycle {
                state.time_in_phase -= cycle;
                state.phase_index = (state.phase_index + 1) % controller.phases.len();
                changed = true;
            } else if was_green && state.time_in_phase >= phase.green_duration {
                changed = true;
            }
        }

        if changed {
            self.refresh(map, network);
        }
    }

    fn refresh(&mut self, map: &Map, network: &Network) {
        self.link_signals.clear();
        self.link_signals.resize(network.link_id_bound(), Signal::Uncontrolled);
        self.green_road_ids.clear();

        let signalised = |link_id: u32| {
            network
                .link(link_id)
                .is_some_and(|info| info.link.link_type == LinkType::TrafficLight)
        };

        for (id, controller) in &map.traffic_lights {
            let mut green_roads = Vec::new();

            for phase in &controller.phases {
                for &link_id in &phase.green_link_ids {
                    if signalised(link_id) {
                        self.link_signals[link_id as usize] = Signal::Red;
                    }
                }
            }

            let state = &self.controllers[id];
            if let Some(phase) = controller.phases.get(state.phase_index) {
                let signal = if state.time_in_phase < phase.green_duration {
                    Signal::Green
                } else {
                    Signal::Yellow
                };
                for &link_id in &phase.green_link_ids {
                    if signalised(link_id) {
                        self.link_signals[link_id as usize] = signal;
                    }
                    if let Some(info) = network.link(link_id) {
                        if let LaneId::Normal(edge, _) = info.in_lane {
                            green_roads.push(map.graph[edge].id);
                        }
                    }
                }
            }

            green_roads.sort_unstable();
            green_roads.dedup();
            self.green_road_ids.insert(*id, green_roads);
        }

        // Signalised links that belong to no phase never turn green.
        for link_id in 0..network.link_id_bound() as u32 {
            if signalised(link_id) && self.link_signals[link_id as usize] == Signal::Uncontrolled {
                self.link_signals[link_id as usize] = Signal::Red;
            }
        }
    }
}
