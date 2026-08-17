//! Platform-independent dictation session behavior.
//!
//! [`SessionMachine`] is the module's interface. Callers supply input edges and completion events;
//! the module returns the small set of effects that platform adapters must perform. It owns all
//! timing, quick-tap, double-tap, lock, and ignored-release behavior.

/// The user action associated with a hotkey.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Dictation,
    Command,
}

/// An event accepted by [`SessionMachine::apply`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Press { action: Action, at_ms: u64 },
    Release { action: Action, at_ms: u64 },
    CancelDeadline { generation: u64 },
    CaptureFailed,
    PipelineFinished,
    PipelineFailed,
    Abort,
}

/// Work the caller must perform after a transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    StartCapture { action: Action },
    StopAndProcess { action: Action },
    DiscardCapture,
    ScheduleCancel { generation: u64, delay_ms: u64 },
    CancelScheduled { generation: u64 },
}

/// Observable state for D-Bus clients, the GTK UI, and behavioral tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Idle,
    Recording { action: Action, locked: bool },
    AwaitingSecondTap,
    Processing { action: Action },
}

/// Result returned after every event, including ignored/idempotent events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transition {
    pub phase: Phase,
    pub effects: Vec<Effect>,
}

#[derive(Clone, Copy, Debug)]
pub struct SessionConfig {
    pub hold_threshold_ms: u64,
    pub double_tap_window_ms: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            hold_threshold_ms: 350,
            double_tap_window_ms: 500,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum State {
    Idle,
    Recording { action: Action, pressed_at_ms: u64 },
    AwaitingSecondTap { generation: u64, deadline_ms: u64 },
    Locked,
    Processing { action: Action },
}

/// Owns the complete hotkey-driven session state machine.
///
/// Duplicate edges and events for the inactive action are intentionally idempotent. Timer
/// generations make late cancellation callbacks harmless. Platform adapters must execute returned
/// effects in order and feed capture/pipeline completion failures back as events.
#[derive(Debug)]
pub struct SessionMachine {
    config: SessionConfig,
    state: State,
    next_generation: u64,
    ignore_next_release: Option<Action>,
}

impl Default for SessionMachine {
    fn default() -> Self {
        Self::new(SessionConfig::default())
    }
}

impl SessionMachine {
    #[must_use]
    pub fn new(config: SessionConfig) -> Self {
        Self {
            config,
            state: State::Idle,
            next_generation: 1,
            ignore_next_release: None,
        }
    }

    #[must_use]
    pub fn phase(&self) -> Phase {
        match self.state {
            State::Idle => Phase::Idle,
            State::Recording { action, .. } => Phase::Recording {
                action,
                locked: false,
            },
            State::AwaitingSecondTap { .. } => Phase::AwaitingSecondTap,
            State::Locked => Phase::Recording {
                action: Action::Dictation,
                locked: true,
            },
            State::Processing { action } => Phase::Processing { action },
        }
    }

    pub fn apply(&mut self, event: Event) -> Transition {
        let effects = match event {
            Event::Press { action, at_ms } => self.press(action, at_ms),
            Event::Release { action, at_ms } => self.release(action, at_ms),
            Event::CancelDeadline { generation } => self.cancel_deadline(generation),
            Event::CaptureFailed => self.capture_failed(),
            Event::PipelineFinished | Event::PipelineFailed => self.pipeline_ended(),
            Event::Abort => self.abort(),
        };
        Transition {
            phase: self.phase(),
            effects,
        }
    }

    fn press(&mut self, action: Action, at_ms: u64) -> Vec<Effect> {
        match self.state {
            State::Idle => {
                self.state = State::Recording {
                    action,
                    pressed_at_ms: at_ms,
                };
                vec![Effect::StartCapture { action }]
            }
            State::Recording { action: active, .. } => {
                // Autorepeat and a competing hotkey cannot start a second capture.
                let _ = active;
                Vec::new()
            }
            State::AwaitingSecondTap {
                generation,
                deadline_ms,
            } if action == Action::Dictation && at_ms <= deadline_ms => {
                self.state = State::Locked;
                vec![Effect::CancelScheduled { generation }]
            }
            State::AwaitingSecondTap { generation, .. } => {
                self.state = State::Recording {
                    action,
                    pressed_at_ms: at_ms,
                };
                vec![
                    Effect::CancelScheduled { generation },
                    Effect::DiscardCapture,
                    Effect::StartCapture { action },
                ]
            }
            State::Locked if action == Action::Dictation => {
                self.state = State::Processing {
                    action: Action::Dictation,
                };
                self.ignore_next_release = Some(Action::Dictation);
                vec![Effect::StopAndProcess {
                    action: Action::Dictation,
                }]
            }
            State::Locked | State::Processing { .. } => Vec::new(),
        }
    }

    fn release(&mut self, action: Action, at_ms: u64) -> Vec<Effect> {
        if self.ignore_next_release == Some(action) {
            self.ignore_next_release = None;
            return Vec::new();
        }

        let State::Recording {
            action: active,
            pressed_at_ms,
        } = self.state
        else {
            return Vec::new();
        };
        if action != active {
            return Vec::new();
        }

        if active == Action::Command
            || at_ms.saturating_sub(pressed_at_ms) >= self.config.hold_threshold_ms
        {
            self.state = State::Processing { action: active };
            return vec![Effect::StopAndProcess { action: active }];
        }

        let generation = self.next_generation;
        self.next_generation = self.next_generation.wrapping_add(1).max(1);
        self.state = State::AwaitingSecondTap {
            generation,
            deadline_ms: at_ms.saturating_add(self.config.double_tap_window_ms),
        };
        vec![Effect::ScheduleCancel {
            generation,
            delay_ms: self.config.double_tap_window_ms,
        }]
    }

    fn cancel_deadline(&mut self, generation: u64) -> Vec<Effect> {
        let State::AwaitingSecondTap {
            generation: active, ..
        } = self.state
        else {
            return Vec::new();
        };
        if generation != active {
            return Vec::new();
        }

        self.state = State::Idle;
        vec![Effect::DiscardCapture]
    }

    fn capture_failed(&mut self) -> Vec<Effect> {
        match self.state {
            State::Recording { .. } | State::AwaitingSecondTap { .. } | State::Locked => {
                self.state = State::Idle;
            }
            State::Idle | State::Processing { .. } => {}
        }
        Vec::new()
    }

    fn pipeline_ended(&mut self) -> Vec<Effect> {
        if matches!(self.state, State::Processing { .. }) {
            self.state = State::Idle;
        }
        Vec::new()
    }

    fn abort(&mut self) -> Vec<Effect> {
        match self.state {
            State::Recording { .. } | State::AwaitingSecondTap { .. } | State::Locked => {
                self.state = State::Idle;
                self.ignore_next_release = None;
                vec![Effect::DiscardCapture]
            }
            State::Idle | State::Processing { .. } => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(machine: &mut SessionMachine, event: Event) -> Transition {
        machine.apply(event)
    }

    #[test]
    fn held_dictation_records_then_processes() {
        let mut machine = SessionMachine::default();

        assert_eq!(
            apply(
                &mut machine,
                Event::Press {
                    action: Action::Dictation,
                    at_ms: 1_000,
                }
            ),
            Transition {
                phase: Phase::Recording {
                    action: Action::Dictation,
                    locked: false,
                },
                effects: vec![Effect::StartCapture {
                    action: Action::Dictation,
                }],
            }
        );
        assert_eq!(
            apply(
                &mut machine,
                Event::Release {
                    action: Action::Dictation,
                    at_ms: 1_500,
                }
            ),
            Transition {
                phase: Phase::Processing {
                    action: Action::Dictation,
                },
                effects: vec![Effect::StopAndProcess {
                    action: Action::Dictation,
                }],
            }
        );
        assert_eq!(
            apply(&mut machine, Event::PipelineFinished).phase,
            Phase::Idle
        );
    }

    #[test]
    fn quick_tap_is_discarded_when_its_deadline_fires() {
        let mut machine = SessionMachine::default();
        apply(
            &mut machine,
            Event::Press {
                action: Action::Dictation,
                at_ms: 100,
            },
        );
        let released = apply(
            &mut machine,
            Event::Release {
                action: Action::Dictation,
                at_ms: 200,
            },
        );
        assert_eq!(released.phase, Phase::AwaitingSecondTap);
        assert_eq!(
            released.effects,
            vec![Effect::ScheduleCancel {
                generation: 1,
                delay_ms: 500,
            }]
        );
        assert_eq!(
            apply(&mut machine, Event::CancelDeadline { generation: 1 }),
            Transition {
                phase: Phase::Idle,
                effects: vec![Effect::DiscardCapture],
            }
        );
    }

    #[test]
    fn double_tap_locks_then_next_press_stops() {
        let mut machine = SessionMachine::default();
        apply(
            &mut machine,
            Event::Press {
                action: Action::Dictation,
                at_ms: 100,
            },
        );
        apply(
            &mut machine,
            Event::Release {
                action: Action::Dictation,
                at_ms: 200,
            },
        );
        assert_eq!(
            apply(
                &mut machine,
                Event::Press {
                    action: Action::Dictation,
                    at_ms: 400,
                }
            ),
            Transition {
                phase: Phase::Recording {
                    action: Action::Dictation,
                    locked: true,
                },
                effects: vec![Effect::CancelScheduled { generation: 1 }],
            }
        );
        // Releasing the second tap does not end a locked session.
        assert!(
            apply(
                &mut machine,
                Event::Release {
                    action: Action::Dictation,
                    at_ms: 450,
                }
            )
            .effects
            .is_empty()
        );
        assert_eq!(
            apply(
                &mut machine,
                Event::Press {
                    action: Action::Dictation,
                    at_ms: 2_000,
                }
            )
            .effects,
            vec![Effect::StopAndProcess {
                action: Action::Dictation,
            }]
        );
        // The physical release paired with the stop press is consumed even if processing ends first.
        apply(&mut machine, Event::PipelineFinished);
        assert!(
            apply(
                &mut machine,
                Event::Release {
                    action: Action::Dictation,
                    at_ms: 2_100,
                }
            )
            .effects
            .is_empty()
        );
        assert_eq!(machine.phase(), Phase::Idle);
    }

    #[test]
    fn command_mode_has_no_quick_tap_delay() {
        let mut machine = SessionMachine::default();
        apply(
            &mut machine,
            Event::Press {
                action: Action::Command,
                at_ms: 100,
            },
        );
        assert_eq!(
            apply(
                &mut machine,
                Event::Release {
                    action: Action::Command,
                    at_ms: 101,
                }
            )
            .effects,
            vec![Effect::StopAndProcess {
                action: Action::Command,
            }]
        );
    }

    #[test]
    fn stale_cancel_timer_cannot_discard_a_locked_session() {
        let mut machine = SessionMachine::default();
        apply(
            &mut machine,
            Event::Press {
                action: Action::Dictation,
                at_ms: 100,
            },
        );
        apply(
            &mut machine,
            Event::Release {
                action: Action::Dictation,
                at_ms: 200,
            },
        );
        apply(
            &mut machine,
            Event::Press {
                action: Action::Dictation,
                at_ms: 300,
            },
        );

        assert!(
            apply(&mut machine, Event::CancelDeadline { generation: 1 })
                .effects
                .is_empty()
        );
        assert_eq!(
            machine.phase(),
            Phase::Recording {
                action: Action::Dictation,
                locked: true,
            }
        );
    }

    #[test]
    fn press_after_expired_double_tap_window_restarts_capture() {
        let mut machine = SessionMachine::default();
        apply(
            &mut machine,
            Event::Press {
                action: Action::Dictation,
                at_ms: 100,
            },
        );
        apply(
            &mut machine,
            Event::Release {
                action: Action::Dictation,
                at_ms: 200,
            },
        );

        assert_eq!(
            apply(
                &mut machine,
                Event::Press {
                    action: Action::Dictation,
                    at_ms: 701,
                }
            )
            .effects,
            vec![
                Effect::CancelScheduled { generation: 1 },
                Effect::DiscardCapture,
                Effect::StartCapture {
                    action: Action::Dictation,
                },
            ]
        );
    }
}
