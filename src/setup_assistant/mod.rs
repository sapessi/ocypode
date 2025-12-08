use std::collections::{HashMap, HashSet};

use crate::telemetry::TelemetryData;

pub mod recommendations;
pub use recommendations::{RecommendationEngine, SetupRecommendation};

/// Types of handling issues that can be detected during a session.
///
/// Each finding type corresponds to a specific driving issue that can be
/// identified through telemetry analysis and mapped to setup recommendations.
#[derive(Debug, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FindingType {
    /// Front tires scrubbing during corner entry, indicating understeer
    CornerEntryUndersteer,
    /// Rear slides out during braking and turn-in
    CornerEntryOversteer,
    /// Instability during corner entry phase
    CornerEntryInstability,
    /// Loss of front grip during mid-corner coasting phase
    MidCornerUndersteer,
    /// Excessive rear rotation during mid-corner
    MidCornerOversteer,
    /// Front tires sliding during corner exit with throttle
    CornerExitUndersteer,
    /// Rear wheelspin during corner exit with throttle
    CornerExitPowerOversteer,
    /// Sudden rear snap during corner exit
    CornerExitSnapOversteer,
    /// Front wheels locking under braking
    FrontBrakeLock,
    /// Rear wheels locking under braking
    RearBrakeLock,
    /// General braking instability
    BrakingInstability,
    /// Tire temperatures consistently above optimal range
    TireOverheating,
    /// Tire temperatures consistently below optimal range
    TireCold,
    /// Suspension bottoming out over bumps or under compression
    BottomingOut,
    /// Excessive trail braking into corners
    ExcessiveTrailbraking,
}

impl std::fmt::Display for FindingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FindingType::CornerEntryUndersteer => write!(f, "Corner Entry Understeer"),
            FindingType::CornerEntryOversteer => write!(f, "Corner Entry Oversteer"),
            FindingType::CornerEntryInstability => write!(f, "Corner Entry Instability"),
            FindingType::MidCornerUndersteer => write!(f, "Mid-Corner Understeer"),
            FindingType::MidCornerOversteer => write!(f, "Mid-Corner Oversteer"),
            FindingType::CornerExitUndersteer => write!(f, "Corner Exit Understeer"),
            FindingType::CornerExitPowerOversteer => write!(f, "Corner Exit Power Oversteer"),
            FindingType::CornerExitSnapOversteer => write!(f, "Corner Exit Snap Oversteer"),
            FindingType::FrontBrakeLock => write!(f, "Front Brake Lock"),
            FindingType::RearBrakeLock => write!(f, "Rear Brake Lock"),
            FindingType::BrakingInstability => write!(f, "Braking Instability"),
            FindingType::TireOverheating => write!(f, "Tire Overheating"),
            FindingType::TireCold => write!(f, "Cold Tires"),
            FindingType::BottomingOut => write!(f, "Bottoming Out"),
            FindingType::ExcessiveTrailbraking => write!(f, "Excessive Trail Braking"),
        }
    }
}

/// A detected handling issue with occurrence tracking and metadata.
///
/// Findings are aggregated from telemetry annotations and track how many times
/// a particular issue has been detected during the session.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Finding {
    /// The type of handling issue detected
    pub finding_type: FindingType,
    /// Number of times this issue has been detected
    pub occurrence_count: usize,
    /// The corner phase where this issue typically occurs
    pub corner_phase: CornerPhase,
    /// Timestamp of the last detection (milliseconds since epoch)
    pub last_detected: u128,
    /// Severity of the issue (0.0 to 1.0)
    pub severity: f32,
}

/// The phase of a corner where a finding was detected.
///
/// Corner phase classification helps provide more specific setup recommendations
/// based on where in the corner the issue occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CornerPhase {
    /// Corner entry phase (braking and initial turn-in)
    Entry,
    /// Mid-corner phase (coasting through apex)
    Mid,
    /// Corner exit phase (throttle application)
    Exit,
    /// Straight sections
    Straight,
    /// Phase could not be determined
    Unknown,
}

impl std::fmt::Display for CornerPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CornerPhase::Entry => write!(f, "Entry"),
            CornerPhase::Mid => write!(f, "Mid-Corner"),
            CornerPhase::Exit => write!(f, "Exit"),
            CornerPhase::Straight => write!(f, "Straight"),
            CornerPhase::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Core state manager for the Setup Assistant feature.
///
/// The SetupAssistant processes telemetry data to extract findings, manages
/// user confirmations, and coordinates with the recommendation engine to
/// provide setup advice.
///
/// # Requirements
///
/// Supports Requirements 1.1, 1.2, 1.3, 1.4 by providing:
/// - Automatic detection and categorization of handling issues
/// - Finding aggregation with occurrence counting
/// - Session management with state clearing
/// - Corner phase classification
pub struct SetupAssistant {
    /// Map of finding types to their current state
    findings: HashMap<FindingType, Finding>,
    /// Set of findings that the user has confirmed
    confirmed_findings: HashSet<FindingType>,
    /// Engine for mapping findings to setup recommendations
    recommendation_engine: RecommendationEngine,
}

impl SetupAssistant {
    /// Create a new SetupAssistant instance.
    pub fn new() -> Self {
        Self {
            findings: HashMap::new(),
            confirmed_findings: HashSet::new(),
            recommendation_engine: RecommendationEngine::new(),
        }
    }

    /// Process telemetry data to extract and aggregate findings.
    ///
    /// This method examines telemetry annotations and converts them into
    /// findings, updating occurrence counts for existing findings.
    ///
    /// # Requirements
    ///
    /// Implements Requirements 1.1, 1.2, 1.4, 1.5:
    /// - Extracts and categorizes findings from annotations
    /// - Aggregates duplicate findings with occurrence counting
    /// - Classifies corner phase from telemetry state
    /// - Classifies slip by context (throttle/brake state)
    pub fn process_telemetry(&mut self, telemetry: &TelemetryData) {
        // Classify corner phase from telemetry state
        let corner_phase = Self::classify_corner_phase(telemetry);

        // Process each annotation
        for annotation in &telemetry.annotations {
            // Map annotation to finding type based on context
            if let Some(finding_type) = Self::annotation_to_finding_type(annotation, telemetry) {
                // Get or create finding
                let finding = self
                    .findings
                    .entry(finding_type.clone())
                    .or_insert(Finding {
                        finding_type: finding_type.clone(),
                        occurrence_count: 0,
                        corner_phase,
                        last_detected: telemetry.timestamp_ms,
                        severity: 0.5,
                    });

                // Aggregate: increment occurrence count
                finding.occurrence_count += 1;
                finding.last_detected = telemetry.timestamp_ms;
            }
        }
    }

    /// Classify the corner phase based on telemetry state.
    ///
    /// Uses brake, throttle, and steering inputs to determine which phase
    /// of the corner the car is in.
    ///
    /// # Requirements
    ///
    /// Implements Requirement 1.4: Corner phase classification
    fn classify_corner_phase(telemetry: &TelemetryData) -> CornerPhase {
        const MIN_BRAKE_THRESHOLD: f32 = 0.1;
        const MIN_THROTTLE_THRESHOLD: f32 = 0.1;
        const MIN_STEERING_THRESHOLD: f32 = 0.05;

        let brake = telemetry.brake.unwrap_or(0.0);
        let throttle = telemetry.throttle.unwrap_or(0.0);
        let steering = telemetry.steering_pct.unwrap_or(0.0).abs();

        // Entry: braking with steering
        if brake > MIN_BRAKE_THRESHOLD && steering > MIN_STEERING_THRESHOLD {
            return CornerPhase::Entry;
        }

        // Exit: throttle with steering
        if throttle > MIN_THROTTLE_THRESHOLD && steering > MIN_STEERING_THRESHOLD {
            return CornerPhase::Exit;
        }

        // Mid: steering but minimal throttle/brake (coasting)
        if steering > MIN_STEERING_THRESHOLD
            && brake < MIN_BRAKE_THRESHOLD
            && throttle < MIN_THROTTLE_THRESHOLD
        {
            return CornerPhase::Mid;
        }

        // Straight: minimal steering
        if steering < MIN_STEERING_THRESHOLD {
            return CornerPhase::Straight;
        }

        CornerPhase::Unknown
    }

    /// Map a telemetry annotation to a finding type based on context.
    ///
    /// Some annotations (like Slip) require additional context from telemetry
    /// to determine the specific finding type.
    ///
    /// # Requirements
    ///
    /// Implements Requirements 1.1, 1.5, 7.1, 7.2, 7.5:
    /// - Maps annotations to finding types
    /// - Classifies slip by throttle/brake context
    fn annotation_to_finding_type(
        annotation: &crate::telemetry::TelemetryAnnotation,
        telemetry: &TelemetryData,
    ) -> Option<FindingType> {
        use crate::telemetry::TelemetryAnnotation;

        match annotation {
            // Scrub always indicates corner entry understeer
            TelemetryAnnotation::Scrub { is_scrubbing, .. } => {
                if *is_scrubbing {
                    Some(FindingType::CornerEntryUndersteer)
                } else {
                    None
                }
            }

            // Slip classification depends on throttle/brake context
            TelemetryAnnotation::Slip {
                is_slip,
                prev_speed,
                cur_speed,
            } => {
                if !*is_slip {
                    return None;
                }

                let brake = telemetry.brake.unwrap_or(0.0);
                let throttle = telemetry.throttle.unwrap_or(0.0);
                let is_speed_decreasing = cur_speed < prev_speed;

                const MIN_BRAKE_THRESHOLD: f32 = 0.1;
                const MIN_THROTTLE_THRESHOLD: f32 = 0.1;

                // Slip during braking = corner entry understeer
                if brake > MIN_BRAKE_THRESHOLD {
                    Some(FindingType::CornerEntryUndersteer)
                }
                // Slip with throttle and no braking = corner exit understeer
                else if throttle > MIN_THROTTLE_THRESHOLD && brake < MIN_BRAKE_THRESHOLD {
                    Some(FindingType::CornerExitUndersteer)
                }
                // Slip during coasting (minimal throttle/brake) with speed loss = mid-corner understeer
                else if throttle < MIN_THROTTLE_THRESHOLD
                    && brake < MIN_BRAKE_THRESHOLD
                    && is_speed_decreasing
                {
                    Some(FindingType::MidCornerUndersteer)
                } else {
                    None
                }
            }

            // Wheelspin indicates corner exit power oversteer
            TelemetryAnnotation::Wheelspin { is_wheelspin, .. } => {
                if *is_wheelspin {
                    Some(FindingType::CornerExitPowerOversteer)
                } else {
                    None
                }
            }

            // Trail brake steering indicates excessive trail braking
            TelemetryAnnotation::TrailbrakeSteering {
                is_excessive_trailbrake_steering,
                ..
            } => {
                if *is_excessive_trailbrake_steering {
                    Some(FindingType::ExcessiveTrailbraking)
                } else {
                    None
                }
            }

            // Entry oversteer
            TelemetryAnnotation::EntryOversteer { is_oversteer, .. } => {
                if *is_oversteer {
                    Some(FindingType::CornerEntryOversteer)
                } else {
                    None
                }
            }

            // Mid-corner understeer
            TelemetryAnnotation::MidCornerUndersteer { is_understeer, .. } => {
                if *is_understeer {
                    Some(FindingType::MidCornerUndersteer)
                } else {
                    None
                }
            }

            // Mid-corner oversteer
            TelemetryAnnotation::MidCornerOversteer { is_oversteer, .. } => {
                if *is_oversteer {
                    Some(FindingType::MidCornerOversteer)
                } else {
                    None
                }
            }

            // Front brake lock
            TelemetryAnnotation::FrontBrakeLock { is_front_lock, .. } => {
                if *is_front_lock {
                    Some(FindingType::FrontBrakeLock)
                } else {
                    None
                }
            }

            // Rear brake lock
            TelemetryAnnotation::RearBrakeLock { is_rear_lock, .. } => {
                if *is_rear_lock {
                    Some(FindingType::RearBrakeLock)
                } else {
                    None
                }
            }

            // Tire overheating
            TelemetryAnnotation::TireOverheating { is_overheating, .. } => {
                if *is_overheating {
                    Some(FindingType::TireOverheating)
                } else {
                    None
                }
            }

            // Tire cold
            TelemetryAnnotation::TireCold { is_cold, .. } => {
                if *is_cold {
                    Some(FindingType::TireCold)
                } else {
                    None
                }
            }

            // Bottoming out
            TelemetryAnnotation::BottomingOut { is_bottoming, .. } => {
                if *is_bottoming {
                    Some(FindingType::BottomingOut)
                } else {
                    None
                }
            }

            // Short shifting is not a setup issue, so we don't map it
            TelemetryAnnotation::ShortShifting { .. } => None,
        }
    }

    /// Toggle the confirmation state of a finding.
    ///
    /// Confirmed findings will have recommendations displayed to the user.
    ///
    /// # Requirements
    ///
    /// Implements Requirement 3.4: Toggle confirmation behavior
    pub fn toggle_confirmation(&mut self, finding_type: FindingType) {
        if self.confirmed_findings.contains(&finding_type) {
            self.confirmed_findings.remove(&finding_type);
        } else {
            self.confirmed_findings.insert(finding_type);
        }
    }

    /// Check if a finding is currently confirmed.
    ///
    /// # Requirements
    ///
    /// Implements Requirement 3.1: Query confirmation state
    pub fn is_confirmed(&self, finding_type: &FindingType) -> bool {
        self.confirmed_findings.contains(finding_type)
    }

    /// Get all current findings.
    ///
    /// # Requirements
    ///
    /// Implements Requirement 2.2: Access to findings for display
    pub fn get_findings(&self) -> &HashMap<FindingType, Finding> {
        &self.findings
    }

    /// Get setup recommendations for all confirmed findings.
    ///
    /// Returns recommendations only for findings that the user has confirmed.
    /// Supports multiple confirmed findings by aggregating all their recommendations.
    /// If no findings are confirmed, returns an empty vector.
    ///
    /// # Requirements
    ///
    /// Implements Requirements 3.3, 3.5, 4.5:
    /// - Returns recommendations only for confirmed findings
    /// - Supports multiple confirmed findings
    /// - Handles unknown corner phases with general recommendations
    pub fn get_recommendations(&self) -> Vec<SetupRecommendation> {
        let mut all_recommendations = Vec::new();

        // Collect recommendations for all confirmed findings
        for confirmed_finding in &self.confirmed_findings {
            let recommendations = self
                .recommendation_engine
                .get_recommendations(confirmed_finding);
            all_recommendations.extend(recommendations);
        }

        all_recommendations
    }

    /// Clear all findings and state for a new session.
    ///
    /// This should be called when a new racing session begins to reset
    /// the analysis state.
    ///
    /// # Requirements
    ///
    /// Implements Requirement 1.3: Session management
    pub fn clear_session(&mut self) {
        self.findings.clear();
        self.confirmed_findings.clear();
    }

    /// Get the current findings for persistence.
    ///
    /// Returns a reference to the findings HashMap for serialization.
    ///
    /// # Requirements
    ///
    /// Implements Requirement 5.3: State persistence
    pub fn get_findings_for_persistence(&self) -> &HashMap<FindingType, Finding> {
        &self.findings
    }

    /// Get the current confirmed findings for persistence.
    ///
    /// Returns a reference to the confirmed findings HashSet for serialization.
    ///
    /// # Requirements
    ///
    /// Implements Requirement 5.3: State persistence
    pub fn get_confirmed_findings_for_persistence(&self) -> &HashSet<FindingType> {
        &self.confirmed_findings
    }

    /// Restore findings from persisted state.
    ///
    /// Replaces the current findings with the provided state.
    ///
    /// # Requirements
    ///
    /// Implements Requirement 5.4: Load saved state on app startup
    pub fn restore_findings(&mut self, findings: HashMap<FindingType, Finding>) {
        self.findings = findings;
    }

    /// Restore confirmed findings from persisted state.
    ///
    /// Replaces the current confirmed findings with the provided state.
    ///
    /// # Requirements
    ///
    /// Implements Requirement 5.4: Load saved state on app startup
    pub fn restore_confirmed_findings(&mut self, confirmed_findings: HashSet<FindingType>) {
        self.confirmed_findings = confirmed_findings;
    }
}

impl Default for SetupAssistant {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_setup_assistant_is_empty() {
        let assistant = SetupAssistant::new();
        assert_eq!(assistant.get_findings().len(), 0);
        assert!(!assistant.is_confirmed(&FindingType::CornerEntryUndersteer));
    }

    #[test]
    fn test_toggle_confirmation() {
        let mut assistant = SetupAssistant::new();
        let finding_type = FindingType::CornerEntryUndersteer;

        // Initially not confirmed
        assert!(!assistant.is_confirmed(&finding_type));

        // Toggle to confirmed
        assistant.toggle_confirmation(finding_type.clone());
        assert!(assistant.is_confirmed(&finding_type));

        // Toggle back to unconfirmed
        assistant.toggle_confirmation(finding_type.clone());
        assert!(!assistant.is_confirmed(&finding_type));
    }

    #[test]
    fn test_clear_session() {
        let mut assistant = SetupAssistant::new();

        // Add a confirmation
        assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);
        assert!(assistant.is_confirmed(&FindingType::CornerEntryUndersteer));

        // Clear session
        assistant.clear_session();

        // Verify everything is cleared
        assert_eq!(assistant.get_findings().len(), 0);
        assert!(!assistant.is_confirmed(&FindingType::CornerEntryUndersteer));
    }

    #[test]
    fn test_clear_session_with_findings() {
        use crate::telemetry::{TelemetryAnnotation, TelemetryData};

        let mut assistant = SetupAssistant::new();

        // Add some findings
        let mut telemetry = TelemetryData::default();
        telemetry.annotations = vec![
            TelemetryAnnotation::Scrub {
                avg_yaw_rate_change: 0.5,
                cur_yaw_rate_change: 0.8,
                is_scrubbing: true,
            },
            TelemetryAnnotation::Wheelspin {
                avg_rpm_increase_per_gear: std::collections::HashMap::new(),
                cur_gear: 2,
                cur_rpm_increase: 500.0,
                is_wheelspin: true,
            },
        ];

        // Process telemetry multiple times
        for _ in 0..5 {
            assistant.process_telemetry(&telemetry);
        }

        // Confirm some findings
        assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);
        assistant.toggle_confirmation(FindingType::CornerExitPowerOversteer);

        // Verify we have findings and confirmations
        assert_eq!(assistant.get_findings().len(), 2);
        assert!(assistant.is_confirmed(&FindingType::CornerEntryUndersteer));
        assert!(assistant.is_confirmed(&FindingType::CornerExitPowerOversteer));

        // Clear session
        assistant.clear_session();

        // Verify everything is cleared
        assert_eq!(assistant.get_findings().len(), 0);
        assert!(!assistant.is_confirmed(&FindingType::CornerEntryUndersteer));
        assert!(!assistant.is_confirmed(&FindingType::CornerExitPowerOversteer));
    }

    #[test]
    fn test_multiple_confirmations() {
        let mut assistant = SetupAssistant::new();

        // Confirm multiple findings
        assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);
        assistant.toggle_confirmation(FindingType::CornerExitPowerOversteer);
        assistant.toggle_confirmation(FindingType::TireOverheating);

        // Verify all are confirmed
        assert!(assistant.is_confirmed(&FindingType::CornerEntryUndersteer));
        assert!(assistant.is_confirmed(&FindingType::CornerExitPowerOversteer));
        assert!(assistant.is_confirmed(&FindingType::TireOverheating));

        // Unconfirm one
        assistant.toggle_confirmation(FindingType::CornerExitPowerOversteer);

        // Verify state
        assert!(assistant.is_confirmed(&FindingType::CornerEntryUndersteer));
        assert!(!assistant.is_confirmed(&FindingType::CornerExitPowerOversteer));
        assert!(assistant.is_confirmed(&FindingType::TireOverheating));
    }

    #[test]
    fn test_corner_phase_equality() {
        assert_eq!(CornerPhase::Entry, CornerPhase::Entry);
        assert_ne!(CornerPhase::Entry, CornerPhase::Exit);
        assert_ne!(CornerPhase::Mid, CornerPhase::Unknown);
    }

    #[test]
    fn test_finding_type_hash_equality() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(FindingType::CornerEntryUndersteer);
        set.insert(FindingType::CornerEntryUndersteer); // Duplicate

        // Should only have one entry
        assert_eq!(set.len(), 1);
        assert!(set.contains(&FindingType::CornerEntryUndersteer));
    }

    #[test]
    fn test_process_telemetry_extracts_scrub_annotation() {
        use crate::telemetry::{TelemetryAnnotation, TelemetryData};

        let mut assistant = SetupAssistant::new();

        let mut telemetry = TelemetryData::default();
        telemetry.annotations = vec![TelemetryAnnotation::Scrub {
            avg_yaw_rate_change: 0.5,
            cur_yaw_rate_change: 0.8,
            is_scrubbing: true,
        }];

        assistant.process_telemetry(&telemetry);

        // Should have one finding
        assert_eq!(assistant.get_findings().len(), 1);
        assert!(
            assistant
                .get_findings()
                .contains_key(&FindingType::CornerEntryUndersteer)
        );
    }

    #[test]
    fn test_process_telemetry_aggregates_duplicate_findings() {
        use crate::telemetry::{TelemetryAnnotation, TelemetryData};

        let mut assistant = SetupAssistant::new();

        let mut telemetry = TelemetryData::default();
        telemetry.annotations = vec![TelemetryAnnotation::Scrub {
            avg_yaw_rate_change: 0.5,
            cur_yaw_rate_change: 0.8,
            is_scrubbing: true,
        }];

        // Process same annotation 3 times
        assistant.process_telemetry(&telemetry);
        assistant.process_telemetry(&telemetry);
        assistant.process_telemetry(&telemetry);

        // Should have one finding with count of 3
        let finding = assistant
            .get_findings()
            .get(&FindingType::CornerEntryUndersteer)
            .unwrap();
        assert_eq!(finding.occurrence_count, 3);
    }

    #[test]
    fn test_classify_corner_phase_entry() {
        use crate::telemetry::TelemetryData;

        let mut telemetry = TelemetryData::default();
        telemetry.brake = Some(0.8);
        telemetry.throttle = Some(0.0);
        telemetry.steering_pct = Some(0.3);

        let phase = SetupAssistant::classify_corner_phase(&telemetry);
        assert_eq!(phase, CornerPhase::Entry);
    }

    #[test]
    fn test_classify_corner_phase_exit() {
        use crate::telemetry::TelemetryData;

        let mut telemetry = TelemetryData::default();
        telemetry.brake = Some(0.0);
        telemetry.throttle = Some(0.7);
        telemetry.steering_pct = Some(0.3);

        let phase = SetupAssistant::classify_corner_phase(&telemetry);
        assert_eq!(phase, CornerPhase::Exit);
    }

    #[test]
    fn test_classify_corner_phase_mid() {
        use crate::telemetry::TelemetryData;

        let mut telemetry = TelemetryData::default();
        telemetry.brake = Some(0.0);
        telemetry.throttle = Some(0.0);
        telemetry.steering_pct = Some(0.3);

        let phase = SetupAssistant::classify_corner_phase(&telemetry);
        assert_eq!(phase, CornerPhase::Mid);
    }

    #[test]
    fn test_classify_corner_phase_straight() {
        use crate::telemetry::TelemetryData;

        let mut telemetry = TelemetryData::default();
        telemetry.brake = Some(0.0);
        telemetry.throttle = Some(0.8);
        telemetry.steering_pct = Some(0.01);

        let phase = SetupAssistant::classify_corner_phase(&telemetry);
        assert_eq!(phase, CornerPhase::Straight);
    }

    #[test]
    fn test_slip_classification_during_braking() {
        use crate::telemetry::{TelemetryAnnotation, TelemetryData};

        let mut telemetry = TelemetryData::default();
        telemetry.brake = Some(0.8);
        telemetry.throttle = Some(0.0);

        let annotation = TelemetryAnnotation::Slip {
            prev_speed: 50.0,
            cur_speed: 48.0,
            is_slip: true,
        };

        let finding_type = SetupAssistant::annotation_to_finding_type(&annotation, &telemetry);
        assert_eq!(finding_type, Some(FindingType::CornerEntryUndersteer));
    }

    #[test]
    fn test_slip_classification_during_throttle() {
        use crate::telemetry::{TelemetryAnnotation, TelemetryData};

        let mut telemetry = TelemetryData::default();
        telemetry.brake = Some(0.0);
        telemetry.throttle = Some(0.8);

        let annotation = TelemetryAnnotation::Slip {
            prev_speed: 50.0,
            cur_speed: 48.0,
            is_slip: true,
        };

        let finding_type = SetupAssistant::annotation_to_finding_type(&annotation, &telemetry);
        assert_eq!(finding_type, Some(FindingType::CornerExitUndersteer));
    }

    #[test]
    fn test_slip_classification_during_coasting() {
        use crate::telemetry::{TelemetryAnnotation, TelemetryData};

        let mut telemetry = TelemetryData::default();
        telemetry.brake = Some(0.0);
        telemetry.throttle = Some(0.0);

        let annotation = TelemetryAnnotation::Slip {
            prev_speed: 50.0,
            cur_speed: 48.0,
            is_slip: true,
        };

        let finding_type = SetupAssistant::annotation_to_finding_type(&annotation, &telemetry);
        assert_eq!(finding_type, Some(FindingType::MidCornerUndersteer));
    }

    #[test]
    fn test_wheelspin_maps_to_power_oversteer() {
        use crate::telemetry::{TelemetryAnnotation, TelemetryData};
        use std::collections::HashMap;

        let telemetry = TelemetryData::default();

        let annotation = TelemetryAnnotation::Wheelspin {
            avg_rpm_increase_per_gear: HashMap::new(),
            cur_gear: 2,
            cur_rpm_increase: 500.0,
            is_wheelspin: true,
        };

        let finding_type = SetupAssistant::annotation_to_finding_type(&annotation, &telemetry);
        assert_eq!(finding_type, Some(FindingType::CornerExitPowerOversteer));
    }

    #[test]
    fn test_get_recommendations_returns_empty_when_no_confirmations() {
        let assistant = SetupAssistant::new();

        // No confirmed findings, should return empty
        let recommendations = assistant.get_recommendations();
        assert_eq!(recommendations.len(), 0);
    }

    #[test]
    fn test_get_recommendations_returns_recommendations_for_confirmed_finding() {
        let mut assistant = SetupAssistant::new();

        // Confirm a finding
        assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);

        // Should return recommendations for the confirmed finding
        let recommendations = assistant.get_recommendations();
        assert!(
            !recommendations.is_empty(),
            "Should have recommendations for confirmed finding"
        );
    }

    #[test]
    fn test_get_recommendations_supports_multiple_confirmed_findings() {
        let mut assistant = SetupAssistant::new();

        // Confirm multiple findings
        assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);
        assistant.toggle_confirmation(FindingType::TireOverheating);
        assistant.toggle_confirmation(FindingType::FrontBrakeLock);

        // Should return recommendations for all confirmed findings
        let recommendations = assistant.get_recommendations();
        assert!(
            !recommendations.is_empty(),
            "Should have recommendations for multiple confirmed findings"
        );
    }

    #[test]
    fn test_get_recommendations_only_returns_confirmed_findings() {
        use crate::telemetry::{TelemetryAnnotation, TelemetryData};

        let mut assistant = SetupAssistant::new();

        // Add some findings through telemetry processing
        let mut telemetry = TelemetryData::default();
        telemetry.annotations = vec![
            TelemetryAnnotation::Scrub {
                avg_yaw_rate_change: 0.5,
                cur_yaw_rate_change: 0.8,
                is_scrubbing: true,
            },
            TelemetryAnnotation::Wheelspin {
                avg_rpm_increase_per_gear: std::collections::HashMap::new(),
                cur_gear: 2,
                cur_rpm_increase: 500.0,
                is_wheelspin: true,
            },
        ];
        assistant.process_telemetry(&telemetry);

        // Verify we have findings but no confirmations
        assert_eq!(assistant.get_findings().len(), 2);
        assert_eq!(
            assistant.get_recommendations().len(),
            0,
            "Should have no recommendations without confirmations"
        );

        // Confirm only one finding
        assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);

        // Should only get recommendations for the confirmed finding
        let recommendations = assistant.get_recommendations();
        assert!(
            !recommendations.is_empty(),
            "Should have recommendations for confirmed finding"
        );
    }

    #[test]
    fn test_get_recommendations_after_unconfirm() {
        let mut assistant = SetupAssistant::new();

        // Confirm a finding
        assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);
        let recommendations_confirmed = assistant.get_recommendations();
        assert!(!recommendations_confirmed.is_empty());

        // Unconfirm it
        assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);
        let recommendations_unconfirmed = assistant.get_recommendations();
        assert_eq!(
            recommendations_unconfirmed.len(),
            0,
            "Should have no recommendations after unconfirming"
        );
    }

    #[test]
    fn test_persistence_methods() {
        use crate::telemetry::{TelemetryAnnotation, TelemetryData};

        let mut assistant = SetupAssistant::new();

        // Add some findings
        let mut telemetry = TelemetryData::default();
        telemetry.annotations = vec![TelemetryAnnotation::Scrub {
            avg_yaw_rate_change: 0.5,
            cur_yaw_rate_change: 0.8,
            is_scrubbing: true,
        }];
        assistant.process_telemetry(&telemetry);

        // Confirm a finding
        assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);

        // Get state for persistence
        let findings = assistant.get_findings_for_persistence().clone();
        let confirmed = assistant.get_confirmed_findings_for_persistence().clone();

        // Verify we have the expected state
        assert_eq!(findings.len(), 1);
        assert_eq!(confirmed.len(), 1);
        assert!(confirmed.contains(&FindingType::CornerEntryUndersteer));

        // Create a new assistant and restore state
        let mut new_assistant = SetupAssistant::new();
        new_assistant.restore_findings(findings);
        new_assistant.restore_confirmed_findings(confirmed);

        // Verify state was restored
        assert_eq!(new_assistant.get_findings().len(), 1);
        assert!(new_assistant.is_confirmed(&FindingType::CornerEntryUndersteer));
        assert!(!new_assistant.get_recommendations().is_empty());
    }

    #[test]
    fn test_persistence_preserves_occurrence_count() {
        use crate::telemetry::{TelemetryAnnotation, TelemetryData};

        let mut assistant = SetupAssistant::new();

        // Add multiple occurrences of the same finding
        let mut telemetry = TelemetryData::default();
        telemetry.annotations = vec![TelemetryAnnotation::Scrub {
            avg_yaw_rate_change: 0.5,
            cur_yaw_rate_change: 0.8,
            is_scrubbing: true,
        }];

        for _ in 0..5 {
            assistant.process_telemetry(&telemetry);
        }

        // Get state for persistence
        let findings = assistant.get_findings_for_persistence().clone();

        // Verify occurrence count
        let finding = findings.get(&FindingType::CornerEntryUndersteer).unwrap();
        assert_eq!(finding.occurrence_count, 5);

        // Restore to new assistant
        let mut new_assistant = SetupAssistant::new();
        new_assistant.restore_findings(findings);

        // Verify occurrence count was preserved
        let restored_finding = new_assistant
            .get_findings()
            .get(&FindingType::CornerEntryUndersteer)
            .unwrap();
        assert_eq!(restored_finding.occurrence_count, 5);
    }

    #[test]
    fn test_persistence_across_window_close_open() {
        use crate::telemetry::{TelemetryAnnotation, TelemetryData};

        let mut assistant = SetupAssistant::new();

        // Simulate a session with multiple findings
        let mut telemetry = TelemetryData::default();
        telemetry.annotations = vec![
            TelemetryAnnotation::Scrub {
                avg_yaw_rate_change: 0.5,
                cur_yaw_rate_change: 0.8,
                is_scrubbing: true,
            },
            TelemetryAnnotation::Wheelspin {
                avg_rpm_increase_per_gear: std::collections::HashMap::new(),
                cur_gear: 2,
                cur_rpm_increase: 500.0,
                is_wheelspin: true,
            },
        ];

        for _ in 0..3 {
            assistant.process_telemetry(&telemetry);
        }

        // Confirm some findings
        assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);
        assistant.toggle_confirmation(FindingType::CornerExitPowerOversteer);

        // Verify initial state
        assert_eq!(assistant.get_findings().len(), 2);
        assert!(assistant.is_confirmed(&FindingType::CornerEntryUndersteer));
        assert!(assistant.is_confirmed(&FindingType::CornerExitPowerOversteer));
        assert!(!assistant.get_recommendations().is_empty());

        // Simulate window close by saving state
        let findings = assistant.get_findings_for_persistence().clone();
        let confirmed = assistant.get_confirmed_findings_for_persistence().clone();

        // Simulate window reopen by restoring state to a new assistant
        let mut new_assistant = SetupAssistant::new();
        new_assistant.restore_findings(findings);
        new_assistant.restore_confirmed_findings(confirmed);

        // Verify all state was preserved
        assert_eq!(new_assistant.get_findings().len(), 2);
        assert!(new_assistant.is_confirmed(&FindingType::CornerEntryUndersteer));
        assert!(new_assistant.is_confirmed(&FindingType::CornerExitPowerOversteer));
        assert!(!new_assistant.get_recommendations().is_empty());

        // Verify occurrence counts were preserved
        let understeer_finding = new_assistant
            .get_findings()
            .get(&FindingType::CornerEntryUndersteer)
            .unwrap();
        assert_eq!(understeer_finding.occurrence_count, 3);

        let oversteer_finding = new_assistant
            .get_findings()
            .get(&FindingType::CornerExitPowerOversteer)
            .unwrap();
        assert_eq!(oversteer_finding.occurrence_count, 3);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::telemetry::{GameSource, TelemetryAnnotation, TelemetryData};
    use proptest::prelude::*;
    use std::collections::HashMap;

    // Generators for property-based testing

    fn arb_finding_type() -> impl Strategy<Value = FindingType> {
        prop_oneof![
            Just(FindingType::CornerEntryUndersteer),
            Just(FindingType::CornerEntryOversteer),
            Just(FindingType::MidCornerUndersteer),
            Just(FindingType::MidCornerOversteer),
            Just(FindingType::CornerExitUndersteer),
            Just(FindingType::CornerExitPowerOversteer),
            Just(FindingType::FrontBrakeLock),
            Just(FindingType::RearBrakeLock),
            Just(FindingType::TireOverheating),
            Just(FindingType::TireCold),
            Just(FindingType::BottomingOut),
            Just(FindingType::ExcessiveTrailbraking),
        ]
    }

    fn arb_annotation() -> impl Strategy<Value = TelemetryAnnotation> {
        prop_oneof![
            // Scrub
            (any::<f32>(), any::<f32>(), any::<bool>()).prop_map(
                |(avg_yaw, cur_yaw, is_scrubbing)| TelemetryAnnotation::Scrub {
                    avg_yaw_rate_change: avg_yaw,
                    cur_yaw_rate_change: cur_yaw,
                    is_scrubbing,
                }
            ),
            // Slip
            (any::<f32>(), any::<f32>(), any::<bool>()).prop_map(
                |(prev_speed, cur_speed, is_slip)| TelemetryAnnotation::Slip {
                    prev_speed,
                    cur_speed,
                    is_slip,
                }
            ),
            // Wheelspin
            (any::<u32>(), any::<f32>(), any::<bool>()).prop_map(
                |(cur_gear, cur_rpm_increase, is_wheelspin)| TelemetryAnnotation::Wheelspin {
                    avg_rpm_increase_per_gear: HashMap::new(),
                    cur_gear,
                    cur_rpm_increase,
                    is_wheelspin,
                }
            ),
            // EntryOversteer
            (any::<f32>(), any::<f32>(), any::<bool>()).prop_map(
                |(expected_yaw, actual_yaw, is_oversteer)| TelemetryAnnotation::EntryOversteer {
                    expected_yaw_rate: expected_yaw,
                    actual_yaw_rate: actual_yaw,
                    is_oversteer,
                }
            ),
            // MidCornerUndersteer
            (any::<f32>(), any::<bool>()).prop_map(|(speed_loss, is_understeer)| {
                TelemetryAnnotation::MidCornerUndersteer {
                    speed_loss,
                    is_understeer,
                }
            }),
            // MidCornerOversteer
            (any::<f32>(), any::<bool>()).prop_map(|(yaw_rate_excess, is_oversteer)| {
                TelemetryAnnotation::MidCornerOversteer {
                    yaw_rate_excess,
                    is_oversteer,
                }
            }),
            // FrontBrakeLock
            (any::<usize>(), any::<bool>()).prop_map(|(abs_count, is_front_lock)| {
                TelemetryAnnotation::FrontBrakeLock {
                    abs_activation_count: abs_count,
                    is_front_lock,
                }
            }),
            // RearBrakeLock
            (any::<usize>(), any::<bool>()).prop_map(|(abs_count, is_rear_lock)| {
                TelemetryAnnotation::RearBrakeLock {
                    abs_activation_count: abs_count,
                    is_rear_lock,
                }
            }),
            // TireOverheating
            (any::<f32>(), any::<f32>(), any::<bool>()).prop_map(
                |(avg_temp, optimal_max, is_overheating)| TelemetryAnnotation::TireOverheating {
                    avg_temp,
                    optimal_max,
                    is_overheating,
                }
            ),
            // TireCold
            (any::<f32>(), any::<f32>(), any::<bool>()).prop_map(
                |(avg_temp, optimal_min, is_cold)| TelemetryAnnotation::TireCold {
                    avg_temp,
                    optimal_min,
                    is_cold,
                }
            ),
            // BottomingOut
            (any::<f32>(), any::<f32>(), any::<bool>()).prop_map(
                |(pitch_change, speed_loss, is_bottoming)| TelemetryAnnotation::BottomingOut {
                    pitch_change,
                    speed_loss,
                    is_bottoming,
                }
            ),
        ]
    }

    fn arb_telemetry_data() -> impl Strategy<Value = TelemetryData> {
        (
            0.0f32..1.0,
            0.0f32..1.0,
            -1.0f32..1.0,
            0.0f32..100.0,
            any::<u128>(),
        )
            .prop_map(|(brake, throttle, steering_pct, speed_mps, timestamp_ms)| {
                TelemetryData {
                    point_no: 0,
                    timestamp_ms,
                    game_source: GameSource::IRacing,
                    brake: Some(brake),
                    throttle: Some(throttle),
                    steering_pct: Some(steering_pct),
                    speed_mps: Some(speed_mps),
                    annotations: Vec::new(),
                    ..Default::default()
                }
            })
    }

    // **Feature: setup-assistant, Property 2: Finding aggregation**
    // **Validates: Requirements 1.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_finding_aggregation(
            base_telemetry in arb_telemetry_data(),
            annotation in arb_annotation(),
            count in 1usize..50
        ) {
            let mut assistant = SetupAssistant::new();

            // Process same annotation multiple times
            for _ in 0..count {
                let mut telemetry = base_telemetry.clone();
                telemetry.annotations = vec![annotation.clone()];
                assistant.process_telemetry(&telemetry);
            }

            // Get the finding type that should have been created
            let finding_type_opt = SetupAssistant::annotation_to_finding_type(&annotation, &base_telemetry);

            if let Some(finding_type) = finding_type_opt {
                // Should have exactly one finding of this type
                let finding = assistant.get_findings().get(&finding_type);
                assert!(finding.is_some(), "Finding should exist for {:?}", finding_type);

                // Occurrence count should match the number of times we processed it
                let finding = finding.unwrap();
                assert_eq!(
                    finding.occurrence_count, count,
                    "Occurrence count should be {} but was {}",
                    count, finding.occurrence_count
                );
            }
        }
    }

    // **Feature: setup-assistant, Property 3: Corner phase classification**
    // **Validates: Requirements 1.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_corner_phase_classification(
            brake in 0.0f32..1.0,
            throttle in 0.0f32..1.0,
            steering_pct in -1.0f32..1.0,
        ) {
            let mut telemetry = TelemetryData::default();
            telemetry.brake = Some(brake);
            telemetry.throttle = Some(throttle);
            telemetry.steering_pct = Some(steering_pct);

            let phase = SetupAssistant::classify_corner_phase(&telemetry);

            const MIN_BRAKE_THRESHOLD: f32 = 0.1;
            const MIN_THROTTLE_THRESHOLD: f32 = 0.1;
            const MIN_STEERING_THRESHOLD: f32 = 0.05;

            let steering_abs = steering_pct.abs();

            // Verify classification is consistent with inputs
            match phase {
                CornerPhase::Entry => {
                    assert!(
                        brake > MIN_BRAKE_THRESHOLD && steering_abs > MIN_STEERING_THRESHOLD,
                        "Entry phase should have brake > {} and steering > {}, got brake={}, steering={}",
                        MIN_BRAKE_THRESHOLD, MIN_STEERING_THRESHOLD, brake, steering_abs
                    );
                }
                CornerPhase::Exit => {
                    assert!(
                        throttle > MIN_THROTTLE_THRESHOLD && steering_abs > MIN_STEERING_THRESHOLD,
                        "Exit phase should have throttle > {} and steering > {}, got throttle={}, steering={}",
                        MIN_THROTTLE_THRESHOLD, MIN_STEERING_THRESHOLD, throttle, steering_abs
                    );
                }
                CornerPhase::Mid => {
                    assert!(
                        steering_abs > MIN_STEERING_THRESHOLD
                            && brake < MIN_BRAKE_THRESHOLD
                            && throttle < MIN_THROTTLE_THRESHOLD,
                        "Mid phase should have steering > {} and brake < {} and throttle < {}, got steering={}, brake={}, throttle={}",
                        MIN_STEERING_THRESHOLD, MIN_BRAKE_THRESHOLD, MIN_THROTTLE_THRESHOLD,
                        steering_abs, brake, throttle
                    );
                }
                CornerPhase::Straight => {
                    assert!(
                        steering_abs < MIN_STEERING_THRESHOLD,
                        "Straight phase should have steering < {}, got steering={}",
                        MIN_STEERING_THRESHOLD, steering_abs
                    );
                }
                CornerPhase::Unknown => {
                    // Unknown is a catch-all for edge cases
                }
            }
        }
    }

    // **Feature: setup-assistant, Property 4: Slip classification by context**
    // **Validates: Requirements 1.5, 7.1, 7.2, 7.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_slip_classification_by_context(
            brake in 0.0f32..1.0,
            throttle in 0.0f32..1.0,
            prev_speed in 0.0f32..100.0,
            cur_speed in 0.0f32..100.0,
        ) {
            let mut telemetry = TelemetryData::default();
            telemetry.brake = Some(brake);
            telemetry.throttle = Some(throttle);

            let annotation = TelemetryAnnotation::Slip {
                prev_speed,
                cur_speed,
                is_slip: true,
            };

            let finding_type = SetupAssistant::annotation_to_finding_type(&annotation, &telemetry);

            const MIN_BRAKE_THRESHOLD: f32 = 0.1;
            const MIN_THROTTLE_THRESHOLD: f32 = 0.1;

            let is_speed_decreasing = cur_speed < prev_speed;

            // Verify classification matches the context
            if brake > MIN_BRAKE_THRESHOLD {
                // Slip during braking should be corner entry understeer
                assert_eq!(
                    finding_type,
                    Some(FindingType::CornerEntryUndersteer),
                    "Slip with brake={} should be CornerEntryUndersteer",
                    brake
                );
            } else if throttle > MIN_THROTTLE_THRESHOLD && brake < MIN_BRAKE_THRESHOLD {
                // Slip with throttle should be corner exit understeer
                assert_eq!(
                    finding_type,
                    Some(FindingType::CornerExitUndersteer),
                    "Slip with throttle={} and brake={} should be CornerExitUndersteer",
                    throttle, brake
                );
            } else if throttle < MIN_THROTTLE_THRESHOLD
                && brake < MIN_BRAKE_THRESHOLD
                && is_speed_decreasing
            {
                // Slip during coasting with speed loss should be mid-corner understeer
                assert_eq!(
                    finding_type,
                    Some(FindingType::MidCornerUndersteer),
                    "Slip with throttle={}, brake={}, speed decreasing should be MidCornerUndersteer",
                    throttle, brake
                );
            }
        }
    }
}
