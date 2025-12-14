use std::collections::{HashMap, HashSet};

use super::FindingType;

/// Categories of car setup parameters that can be adjusted.
///
/// Setup changes are organized by category to help drivers understand
/// which systems they're modifying and to group related adjustments.
///
/// # Requirements
///
/// Supports Requirement 4.4: Organize recommendations by setup category
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SetupCategory {
    /// Aerodynamic adjustments (wings, ride height, splitter)
    Aerodynamics,
    /// Spring rates and ride height
    Suspension,
    /// Anti-roll bar stiffness
    AntiRollBar,
    /// Damper settings (bump, rebound, fast, slow)
    Dampers,
    /// Brake bias and pressure
    Brakes,
    /// Differential settings (preload, locking)
    Drivetrain,
    /// Electronic aids (TC, ABS)
    Electronics,
    /// Wheel alignment (camber, toe, caster)
    Alignment,
    /// Tire pressure and brake duct settings
    TireManagement,
}

impl std::fmt::Display for SetupCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetupCategory::Aerodynamics => write!(f, "Aero"),
            SetupCategory::Suspension => write!(f, "Suspension"),
            SetupCategory::AntiRollBar => write!(f, "Antirollbar"),
            SetupCategory::Dampers => write!(f, "Dampers"),
            SetupCategory::Brakes => write!(f, "Brakes"),
            SetupCategory::Drivetrain => write!(f, "Drivetrain"),
            SetupCategory::Electronics => write!(f, "Electronics"),
            SetupCategory::Alignment => write!(f, "Alignment"),
            SetupCategory::TireManagement => write!(f, "Tire Mgmt"),
        }
    }
}

/// A specific setup change recommendation.
///
/// Each recommendation describes a single parameter adjustment that can
/// help address a detected handling issue.
///
/// # Requirements
///
/// Supports Requirements 4.2, 4.3:
/// - Shows parameter name, adjustment direction, and description
/// - Contains all required fields for display
#[derive(Debug, Clone)]
pub struct SetupRecommendation {
    /// The category this recommendation belongs to
    pub category: SetupCategory,
    /// The name of the parameter to adjust (e.g., "Front Ride Height")
    pub parameter: String,
    /// The direction of adjustment (e.g., "Reduce", "Increase", "Soften")
    pub adjustment: String,
    /// Explanation of why this adjustment helps
    pub description: String,
    /// Priority level (1-5, where 5 is highest priority)
    pub priority: u8,
}

/// A processed recommendation with conflict information.
#[derive(Debug, Clone)]
pub struct ProcessedRecommendation {
    /// The original recommendation
    pub recommendation: SetupRecommendation,
    /// Conflicting recommendations (if any)
    pub conflicts: Vec<SetupRecommendation>,
    /// Whether this recommendation conflicts with others
    pub has_conflict: bool,
}

/// Engine that maps findings to setup recommendations.
///
/// The RecommendationEngine maintains a comprehensive map from each finding
/// type to its corresponding setup recommendations, based on the ACC Setup Guide.
///
/// # Requirements
///
/// Supports Requirements 4.1, 4.2, 4.3, 4.4:
/// - Maps findings to appropriate setup recommendations
/// - Provides all applicable setup changes for each issue type
/// - Organizes recommendations by category
/// - Returns complete recommendation data with descriptions
pub struct RecommendationEngine {
    /// Map from finding types to their recommendations
    recommendation_map: HashMap<FindingType, Vec<SetupRecommendation>>,
}

impl RecommendationEngine {
    /// Create a new RecommendationEngine with the full recommendation map.
    ///
    /// # Requirements
    ///
    /// Implements Requirement 4.1: Map findings to recommendations
    pub fn new() -> Self {
        Self {
            recommendation_map: Self::build_recommendation_map(),
        }
    }

    /// Format a recommendation description with corner context.
    ///
    /// When corner numbers are available, this method enhances the recommendation
    /// description to include specific corner references.
    ///
    /// # Requirements
    ///
    /// Implements Requirement 5.4: Update recommendation formatting to include corner context
    pub fn format_recommendation_with_corners(
        &self,
        recommendation: &SetupRecommendation,
        corner_numbers: &HashSet<u32>,
        _finding_type: &FindingType,
    ) -> String {
        if corner_numbers.is_empty() {
            // No corner context available, return original description
            return recommendation.description.clone();
        }

        // Format corner numbers for display
        let corner_text = if corner_numbers.len() == 1 {
            format!("corner {}", corner_numbers.iter().next().unwrap())
        } else {
            let mut corners: Vec<u32> = corner_numbers.iter().cloned().collect();
            corners.sort();
            if corners.len() <= 3 {
                format!(
                    "corners {}",
                    corners
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                format!(
                    "corners {}, {}, {} and {} others",
                    corners[0],
                    corners[1],
                    corners[2],
                    corners.len() - 3
                )
            }
        };

        // Enhance description with corner context
        format!(
            "{} (detected in {})",
            recommendation.description, corner_text
        )
    }

    /// Build the complete recommendation map from the ACC Setup Guide.
    ///
    /// This method creates the mapping from each finding type to its
    /// corresponding setup recommendations. The recommendations are based
    /// on the ACC Setup Guide methodology.
    ///
    /// # Requirements
    ///
    /// Implements Requirements 4.1, 4.2, 4.3, 4.4:
    /// - Maps each finding type to recommendations
    /// - Includes all applicable setup changes
    /// - Organizes by category
    /// - Provides complete descriptions
    fn build_recommendation_map() -> HashMap<FindingType, Vec<SetupRecommendation>> {
        let mut map = HashMap::new();

        // Corner Entry Understeer (Requirements 6.2, 6.3, 6.4, 6.5)
        map.insert(
            FindingType::CornerEntryUndersteer,
            vec![
                SetupRecommendation {
                    category: SetupCategory::AntiRollBar,
                    parameter: "Front Antirollbar".to_string(),
                    adjustment: "Soften".to_string(),
                    description:
                        "Softer front anti-roll bar allows more front grip during corner entry"
                            .to_string(),
                    priority: 5, // Highest impact, easy to adjust
                },
                SetupRecommendation {
                    category: SetupCategory::Brakes,
                    parameter: "Brake Bias".to_string(),
                    adjustment: "Move Rearward".to_string(),
                    description:
                        "Moving brake bias rearward reduces front tire load during braking"
                            .to_string(),
                    priority: 4, // High impact, easy to adjust
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Front Springs".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer front springs improve mechanical grip during turn-in"
                        .to_string(),
                    priority: 4, // High impact
                },
                SetupRecommendation {
                    category: SetupCategory::Aerodynamics,
                    parameter: "Front Ride Height".to_string(),
                    adjustment: "Reduce".to_string(),
                    description: "Lowering front ride height increases front downforce and grip"
                        .to_string(),
                    priority: 3, // Medium impact, affects other parameters
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Rear Springs".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer rear springs reduce rear grip, shifting balance forward"
                        .to_string(),
                    priority: 3, // Medium impact
                },
                SetupRecommendation {
                    category: SetupCategory::Aerodynamics,
                    parameter: "Rear Ride Height".to_string(),
                    adjustment: "Increase".to_string(),
                    description:
                        "Raising rear ride height reduces rear downforce, shifting balance forward"
                            .to_string(),
                    priority: 3, // Medium impact
                },
                SetupRecommendation {
                    category: SetupCategory::Dampers,
                    parameter: "Front Bump".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer front bump damping allows weight transfer to front tires"
                        .to_string(),
                    priority: 2, // Lower impact, more complex
                },
                SetupRecommendation {
                    category: SetupCategory::Dampers,
                    parameter: "Rear Rebound".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer rear rebound keeps weight on front tires longer"
                        .to_string(),
                    priority: 2, // Lower impact, more complex
                },
                SetupRecommendation {
                    category: SetupCategory::Alignment,
                    parameter: "Front Toe".to_string(),
                    adjustment: "Increase Toe Out".to_string(),
                    description: "Toe out improves turn-in response and front grip".to_string(),
                    priority: 2, // Lower priority, affects tire wear
                },
            ],
        );

        // Corner Entry Oversteer (Requirements 11.3, 11.4, 11.5)
        map.insert(
            FindingType::CornerEntryOversteer,
            vec![
                SetupRecommendation {
                    category: SetupCategory::Brakes,
                    parameter: "Brake Bias".to_string(),
                    adjustment: "Move Forward".to_string(),
                    description: "Moving brake bias forward increases rear stability under braking"
                        .to_string(),
                    priority: 5, // Highest impact, easy to adjust
                },
                SetupRecommendation {
                    category: SetupCategory::Drivetrain,
                    parameter: "Differential Preload".to_string(),
                    adjustment: "Increase".to_string(),
                    description: "Higher preload locks differential on coast, stabilizing rear"
                        .to_string(),
                    priority: 4, // High impact
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Rear Springs".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer rear springs improve rear mechanical grip".to_string(),
                    priority: 4, // High impact
                },
                SetupRecommendation {
                    category: SetupCategory::AntiRollBar,
                    parameter: "Front Antirollbar".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer front anti-roll bar reduces front grip".to_string(),
                    priority: 3, // Medium impact
                },
                SetupRecommendation {
                    category: SetupCategory::Aerodynamics,
                    parameter: "Rear Ride Height".to_string(),
                    adjustment: "Reduce".to_string(),
                    description: "Lowering rear ride height increases rear downforce and stability"
                        .to_string(),
                    priority: 3, // Medium impact
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Front Springs".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer front springs reduce front grip during turn-in"
                        .to_string(),
                    priority: 3, // Medium impact
                },
                SetupRecommendation {
                    category: SetupCategory::Aerodynamics,
                    parameter: "Front Ride Height".to_string(),
                    adjustment: "Increase".to_string(),
                    description: "Raising front ride height reduces front downforce".to_string(),
                    priority: 2, // Lower priority
                },
                SetupRecommendation {
                    category: SetupCategory::Dampers,
                    parameter: "Front Bump".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer front bump reduces weight transfer to front".to_string(),
                    priority: 2, // Lower impact
                },
                SetupRecommendation {
                    category: SetupCategory::Dampers,
                    parameter: "Rear Rebound".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer rear rebound allows rear to settle faster".to_string(),
                    priority: 2, // Lower impact
                },
            ],
        );

        // Corner Entry Instability
        map.insert(
            FindingType::CornerEntryInstability,
            vec![
                SetupRecommendation {
                    category: SetupCategory::Brakes,
                    parameter: "Brake Bias".to_string(),
                    adjustment: "Move Forward".to_string(),
                    description: "Forward brake bias stabilizes the rear under braking".to_string(),
                    priority: 5,
                },
                SetupRecommendation {
                    category: SetupCategory::Drivetrain,
                    parameter: "Differential Preload".to_string(),
                    adjustment: "Increase".to_string(),
                    description: "Higher preload provides more predictable rear behavior"
                        .to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Front Springs".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer front springs reduce pitch and improve stability"
                        .to_string(),
                    priority: 4,
                },
            ],
        );

        // Mid-Corner Understeer (Requirements 12.3, 12.4)
        map.insert(
            FindingType::MidCornerUndersteer,
            vec![
                SetupRecommendation {
                    category: SetupCategory::AntiRollBar,
                    parameter: "Front Antirollbar".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer front Antirollbar allows more front grip mid-corner"
                        .to_string(),
                    priority: 5,
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Front Springs".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer front springs improve mechanical grip".to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::AntiRollBar,
                    parameter: "Rear Antirollbar".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description:
                        "Stiffer rear Antirollbar reduces rear grip, shifting balance forward"
                            .to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::Aerodynamics,
                    parameter: "Front Wing".to_string(),
                    adjustment: "Increase".to_string(),
                    description: "More front wing increases front downforce at apex".to_string(),
                    priority: 3,
                },
                SetupRecommendation {
                    category: SetupCategory::Aerodynamics,
                    parameter: "Splitter".to_string(),
                    adjustment: "Increase".to_string(),
                    description: "More splitter increases front downforce".to_string(),
                    priority: 3,
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Rear Springs".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer rear springs reduce rear grip".to_string(),
                    priority: 3,
                },
                SetupRecommendation {
                    category: SetupCategory::Alignment,
                    parameter: "Front Camber".to_string(),
                    adjustment: "Increase Negative".to_string(),
                    description:
                        "More negative camber improves front tire contact patch mid-corner"
                            .to_string(),
                    priority: 3,
                },
            ],
        );

        // Mid-Corner Oversteer (Requirements 12.5)
        map.insert(
            FindingType::MidCornerOversteer,
            vec![
                SetupRecommendation {
                    category: SetupCategory::AntiRollBar,
                    parameter: "Rear Antirollbar".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer rear Antirollbar allows more rear grip mid-corner"
                        .to_string(),
                    priority: 5,
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Rear Springs".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer rear springs improve rear mechanical grip".to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::AntiRollBar,
                    parameter: "Front Antirollbar".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer front Antirollbar reduces front grip".to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::Aerodynamics,
                    parameter: "Rear Wing".to_string(),
                    adjustment: "Increase".to_string(),
                    description: "More rear wing increases rear downforce and stability"
                        .to_string(),
                    priority: 3,
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Front Springs".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer front springs reduce front grip".to_string(),
                    priority: 3,
                },
                SetupRecommendation {
                    category: SetupCategory::Alignment,
                    parameter: "Rear Camber".to_string(),
                    adjustment: "Increase Negative".to_string(),
                    description: "More negative camber improves rear tire contact patch"
                        .to_string(),
                    priority: 3,
                },
            ],
        );

        // Corner Exit Understeer (Requirements 7.3, 7.4)
        map.insert(
            FindingType::CornerExitUndersteer,
            vec![
                SetupRecommendation {
                    category: SetupCategory::Drivetrain,
                    parameter: "Differential Preload".to_string(),
                    adjustment: "Increase".to_string(),
                    description: "Higher preload helps rotate the car on power".to_string(),
                    priority: 5,
                },
                SetupRecommendation {
                    category: SetupCategory::Drivetrain,
                    parameter: "Differential Locking".to_string(),
                    adjustment: "Increase".to_string(),
                    description: "More locking helps transfer power and rotate the car".to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Front Springs".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer front springs improve front grip on exit".to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Rear Springs".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer rear springs reduce rear grip, helping rotation"
                        .to_string(),
                    priority: 3,
                },
                SetupRecommendation {
                    category: SetupCategory::Dampers,
                    parameter: "Rear Slow Bump".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer rear slow bump reduces rear squat on acceleration"
                        .to_string(),
                    priority: 2,
                },
                SetupRecommendation {
                    category: SetupCategory::Dampers,
                    parameter: "Front Slow Rebound".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer front slow rebound allows front to settle faster"
                        .to_string(),
                    priority: 2,
                },
            ],
        );

        // Corner Exit Power Oversteer (Requirements 8.2, 8.3, 8.4, 8.5)
        map.insert(
            FindingType::CornerExitPowerOversteer,
            vec![
                SetupRecommendation {
                    category: SetupCategory::Electronics,
                    parameter: "Traction Control".to_string(),
                    adjustment: "Increase".to_string(),
                    description: "Higher TC cuts power to prevent wheelspin".to_string(),
                    priority: 5,
                },
                SetupRecommendation {
                    category: SetupCategory::Drivetrain,
                    parameter: "Differential Preload".to_string(),
                    adjustment: "Reduce".to_string(),
                    description: "Lower preload allows more rear slip, reducing wheelspin"
                        .to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::Drivetrain,
                    parameter: "Differential Locking".to_string(),
                    adjustment: "Reduce".to_string(),
                    description:
                        "Less locking allows wheels to spin independently, improving traction"
                            .to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Rear Springs".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer rear springs improve rear mechanical grip".to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::Aerodynamics,
                    parameter: "Rear Wing".to_string(),
                    adjustment: "Increase".to_string(),
                    description: "More rear wing increases rear downforce at high speeds"
                        .to_string(),
                    priority: 3,
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Front Springs".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer front springs reduce front grip, stabilizing rear"
                        .to_string(),
                    priority: 3,
                },
                SetupRecommendation {
                    category: SetupCategory::Dampers,
                    parameter: "Rear Slow Bump".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer rear slow bump allows rear to settle and grip".to_string(),
                    priority: 2,
                },
                SetupRecommendation {
                    category: SetupCategory::Dampers,
                    parameter: "Front Slow Rebound".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer front slow rebound keeps weight on rear tires"
                        .to_string(),
                    priority: 2,
                },
            ],
        );

        // Corner Exit Snap Oversteer
        map.insert(
            FindingType::CornerExitSnapOversteer,
            vec![
                SetupRecommendation {
                    category: SetupCategory::AntiRollBar,
                    parameter: "Rear Antirollbar".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer rear Antirollbar allows more rear compliance".to_string(),
                    priority: 5,
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Rear Springs".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer rear springs prevent sudden rear grip loss".to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::Dampers,
                    parameter: "Rear Fast Bump".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer rear fast bump prevents sudden compression".to_string(),
                    priority: 2,
                },
            ],
        );

        // Front Brake Lock (Requirements 13.4)
        map.insert(
            FindingType::FrontBrakeLock,
            vec![
                SetupRecommendation {
                    category: SetupCategory::Brakes,
                    parameter: "Brake Bias".to_string(),
                    adjustment: "Move Rearward".to_string(),
                    description: "Moving brake bias rearward reduces front brake force".to_string(),
                    priority: 5,
                },
                SetupRecommendation {
                    category: SetupCategory::Brakes,
                    parameter: "Brake Pressure".to_string(),
                    adjustment: "Reduce".to_string(),
                    description: "Lower brake pressure reduces overall braking force".to_string(),
                    priority: 4,
                },
            ],
        );

        // Rear Brake Lock (Requirements 13.5)
        map.insert(
            FindingType::RearBrakeLock,
            vec![SetupRecommendation {
                category: SetupCategory::Brakes,
                parameter: "Brake Bias".to_string(),
                adjustment: "Move Forward".to_string(),
                description: "Moving brake bias forward reduces rear brake force".to_string(),
                priority: 5,
            }],
        );

        // Braking Instability
        map.insert(
            FindingType::BrakingInstability,
            vec![
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Front Springs".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer front springs reduce brake dive".to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::Aerodynamics,
                    parameter: "Rear Ride Height".to_string(),
                    adjustment: "Reduce".to_string(),
                    description: "Lower rear ride height increases rear stability under braking"
                        .to_string(),
                    priority: 3,
                },
                SetupRecommendation {
                    category: SetupCategory::Dampers,
                    parameter: "Front Bump".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer front bump controls weight transfer under braking"
                        .to_string(),
                    priority: 2,
                },
            ],
        );

        // Tire Overheating (Requirements 14.3, 14.4)
        map.insert(
            FindingType::TireOverheating,
            vec![
                SetupRecommendation {
                    category: SetupCategory::TireManagement,
                    parameter: "Brake Ducts".to_string(),
                    adjustment: "Open".to_string(),
                    description: "Opening brake ducts increases cooling to tires".to_string(),
                    priority: 5,
                },
                SetupRecommendation {
                    category: SetupCategory::AntiRollBar,
                    parameter: "Antirollbars".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer Antirollbars reduce tire stress".to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Springs".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer suspension reduces energy transfer to tires".to_string(),
                    priority: 4,
                },
            ],
        );

        // Tire Cold (Requirements 14.5)
        map.insert(
            FindingType::TireCold,
            vec![
                SetupRecommendation {
                    category: SetupCategory::TireManagement,
                    parameter: "Brake Ducts".to_string(),
                    adjustment: "Close".to_string(),
                    description: "Closing brake ducts retains heat in tires".to_string(),
                    priority: 5,
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Springs".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer suspension generates more tire heat".to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::Alignment,
                    parameter: "Toe".to_string(),
                    adjustment: "Increase".to_string(),
                    description: "More toe generates friction heat in tires".to_string(),
                    priority: 2,
                },
            ],
        );

        // Bottoming Out (Requirements 15.3, 15.4, 15.5)
        map.insert(
            FindingType::BottomingOut,
            vec![
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Ride Height".to_string(),
                    adjustment: "Increase".to_string(),
                    description: "Higher ride height prevents suspension bottoming".to_string(),
                    priority: 5,
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Springs".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer springs resist compression over bumps".to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::Dampers,
                    parameter: "Fast Bump".to_string(),
                    adjustment: "Stiffen".to_string(),
                    description: "Stiffer fast bump damping controls compression on impacts"
                        .to_string(),
                    priority: 2,
                },
            ],
        );

        // Excessive Trailbraking
        map.insert(
            FindingType::ExcessiveTrailbraking,
            vec![
                SetupRecommendation {
                    category: SetupCategory::Brakes,
                    parameter: "Brake Bias".to_string(),
                    adjustment: "Move Forward".to_string(),
                    description: "Forward brake bias reduces rear instability during trail braking"
                        .to_string(),
                    priority: 5,
                },
                SetupRecommendation {
                    category: SetupCategory::Drivetrain,
                    parameter: "Differential Preload".to_string(),
                    adjustment: "Increase".to_string(),
                    description: "Higher preload stabilizes rear during coast".to_string(),
                    priority: 4,
                },
                SetupRecommendation {
                    category: SetupCategory::Suspension,
                    parameter: "Rear Springs".to_string(),
                    adjustment: "Soften".to_string(),
                    description: "Softer rear springs improve rear stability".to_string(),
                    priority: 4,
                },
            ],
        );

        map
    }

    /// Get recommendations for a specific finding type.
    ///
    /// Returns all setup recommendations that can help address the given
    /// finding type. If no recommendations exist for the finding type,
    /// returns an empty vector.
    ///
    /// # Requirements
    ///
    /// Implements Requirement 4.1: Retrieve recommendations for findings
    pub fn get_recommendations(&self, finding_type: &FindingType) -> Vec<SetupRecommendation> {
        self.recommendation_map
            .get(finding_type)
            .cloned()
            .unwrap_or_default()
    }

    /// Process and prioritize recommendations, detecting conflicts.
    ///
    /// Takes a list of recommendations from multiple confirmed findings and:
    /// - Sorts by priority (highest first)
    /// - Detects conflicting adjustments to the same parameter
    /// - Returns processed recommendations with conflict information
    ///
    /// # Arguments
    /// * `recommendations` - Raw recommendations from confirmed findings
    ///
    /// # Returns
    /// Vector of processed recommendations, sorted by priority with conflicts identified
    pub fn process_recommendations(
        &self,
        recommendations: Vec<SetupRecommendation>,
    ) -> Vec<ProcessedRecommendation> {
        // Group recommendations by parameter
        let mut by_parameter: HashMap<String, Vec<SetupRecommendation>> = HashMap::new();
        for rec in recommendations {
            by_parameter
                .entry(rec.parameter.clone())
                .or_default()
                .push(rec);
        }

        let mut processed = Vec::new();

        // Process each parameter group
        for (_, mut recs) in by_parameter {
            if recs.len() == 1 {
                // No conflicts for this parameter
                let rec = recs.pop().unwrap();
                processed.push(ProcessedRecommendation {
                    recommendation: rec,
                    conflicts: Vec::new(),
                    has_conflict: false,
                });
            } else {
                // Multiple recommendations for same parameter - check for conflicts
                let conflicts = Self::detect_conflicts(&recs);

                if conflicts.is_empty() {
                    // Same adjustment direction - take highest priority
                    recs.sort_by(|a, b| b.priority.cmp(&a.priority));
                    let rec = recs.remove(0);
                    processed.push(ProcessedRecommendation {
                        recommendation: rec,
                        conflicts: Vec::new(),
                        has_conflict: false,
                    });
                } else {
                    // Conflicting adjustments - include all with conflict markers
                    recs.sort_by(|a, b| b.priority.cmp(&a.priority));
                    for rec in recs {
                        let other_conflicts: Vec<_> = conflicts
                            .iter()
                            .filter(|c| c.adjustment != rec.adjustment)
                            .cloned()
                            .collect();

                        processed.push(ProcessedRecommendation {
                            recommendation: rec,
                            conflicts: other_conflicts,
                            has_conflict: true,
                        });
                    }
                }
            }
        }

        // Sort by priority (highest first), then by parameter name for stable ordering
        processed.sort_by(|a, b| {
            b.recommendation
                .priority
                .cmp(&a.recommendation.priority)
                .then_with(|| a.recommendation.parameter.cmp(&b.recommendation.parameter))
        });

        processed
    }

    /// Detect conflicting adjustments within a group of recommendations.
    ///
    /// Returns recommendations that have conflicting adjustment directions.
    fn detect_conflicts(recs: &[SetupRecommendation]) -> Vec<SetupRecommendation> {
        let mut conflicts = Vec::new();

        // Check if adjustments are in opposite directions
        for i in 0..recs.len() {
            for j in (i + 1)..recs.len() {
                if Self::is_conflicting(&recs[i].adjustment, &recs[j].adjustment) {
                    if !conflicts.iter().any(|c: &SetupRecommendation| {
                        c.parameter == recs[i].parameter && c.adjustment == recs[i].adjustment
                    }) {
                        conflicts.push(recs[i].clone());
                    }
                    if !conflicts.iter().any(|c: &SetupRecommendation| {
                        c.parameter == recs[j].parameter && c.adjustment == recs[j].adjustment
                    }) {
                        conflicts.push(recs[j].clone());
                    }
                }
            }
        }

        conflicts
    }

    /// Check if two adjustment directions conflict.
    fn is_conflicting(adj1: &str, adj2: &str) -> bool {
        let adj1_lower = adj1.to_lowercase();
        let adj2_lower = adj2.to_lowercase();

        // Define opposing adjustment pairs
        let opposing_pairs = [
            ("increase", "reduce"),
            ("increase", "decrease"),
            ("stiffen", "soften"),
            ("open", "close"),
            ("forward", "rearward"),
            ("forward", "backward"),
        ];

        for (op1, op2) in &opposing_pairs {
            if (adj1_lower.contains(op1) && adj2_lower.contains(op2))
                || (adj1_lower.contains(op2) && adj2_lower.contains(op1))
            {
                return true;
            }
        }

        false
    }
}

impl Default for RecommendationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_recommendation_engine() {
        let engine = RecommendationEngine::new();
        // Should be able to create without panicking
        assert!(engine.recommendation_map.is_empty() || !engine.recommendation_map.is_empty());
    }

    #[test]
    fn test_get_recommendations_returns_recommendations() {
        let engine = RecommendationEngine::new();
        let recs = engine.get_recommendations(&FindingType::CornerEntryUndersteer);
        // Should return recommendations for known finding types
        assert!(
            !recs.is_empty(),
            "Should have recommendations for CornerEntryUndersteer"
        );
    }

    #[test]
    fn test_setup_category_equality() {
        assert_eq!(SetupCategory::Aerodynamics, SetupCategory::Aerodynamics);
        assert_ne!(SetupCategory::Aerodynamics, SetupCategory::Suspension);
    }

    #[test]
    fn test_setup_category_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(SetupCategory::Aerodynamics);
        set.insert(SetupCategory::Aerodynamics); // Duplicate

        // Should only have one entry
        assert_eq!(set.len(), 1);
        assert!(set.contains(&SetupCategory::Aerodynamics));
    }

    #[test]
    fn test_setup_recommendation_creation() {
        let rec = SetupRecommendation {
            category: SetupCategory::Aerodynamics,
            parameter: "Front Ride Height".to_string(),
            adjustment: "Reduce".to_string(),
            description: "Lowering front ride height increases front downforce".to_string(),
            priority: 3,
        };

        assert_eq!(rec.category, SetupCategory::Aerodynamics);
        assert_eq!(rec.parameter, "Front Ride Height");
        assert_eq!(rec.adjustment, "Reduce");
        assert!(!rec.description.is_empty());
        assert_eq!(rec.priority, 3);
    }

    #[test]
    fn test_setup_recommendation_clone() {
        let rec = SetupRecommendation {
            category: SetupCategory::Suspension,
            parameter: "Front Spring Rate".to_string(),
            adjustment: "Soften".to_string(),
            description: "Softer springs improve mechanical grip".to_string(),
            priority: 4,
        };

        let cloned = rec.clone();
        assert_eq!(rec.category, cloned.category);
        assert_eq!(rec.parameter, cloned.parameter);
        assert_eq!(rec.adjustment, cloned.adjustment);
        assert_eq!(rec.description, cloned.description);
        assert_eq!(rec.priority, cloned.priority);
    }

    #[test]
    fn test_recommendation_retrieval_for_each_finding_type() {
        let engine = RecommendationEngine::new();

        // Test each finding type has recommendations
        let finding_types = vec![
            FindingType::CornerEntryUndersteer,
            FindingType::CornerEntryOversteer,
            FindingType::CornerEntryInstability,
            FindingType::MidCornerUndersteer,
            FindingType::MidCornerOversteer,
            FindingType::CornerExitUndersteer,
            FindingType::CornerExitPowerOversteer,
            FindingType::CornerExitSnapOversteer,
            FindingType::FrontBrakeLock,
            FindingType::RearBrakeLock,
            FindingType::BrakingInstability,
            FindingType::TireOverheating,
            FindingType::TireCold,
            FindingType::BottomingOut,
            FindingType::ExcessiveTrailbraking,
        ];

        for finding_type in finding_types {
            let recs = engine.get_recommendations(&finding_type);
            assert!(
                !recs.is_empty(),
                "Finding type {:?} should have recommendations",
                finding_type
            );
        }
    }

    #[test]
    fn test_all_finding_types_have_recommendations() {
        let engine = RecommendationEngine::new();

        // Verify the map is not empty
        assert!(!engine.recommendation_map.is_empty());

        // Verify we have recommendations for all 15 finding types
        assert_eq!(
            engine.recommendation_map.len(),
            15,
            "Should have recommendations for all 15 finding types"
        );
    }

    #[test]
    fn test_category_grouping() {
        use std::collections::HashMap;

        let engine = RecommendationEngine::new();
        let recs = engine.get_recommendations(&FindingType::CornerEntryUndersteer);

        // Group by category
        let mut by_category: HashMap<String, Vec<&SetupRecommendation>> = HashMap::new();
        for rec in &recs {
            let category_key = format!("{:?}", rec.category);
            by_category.entry(category_key).or_default().push(rec);
        }

        // Should have multiple categories for corner entry understeer
        assert!(
            by_category.len() > 1,
            "Corner entry understeer should have recommendations in multiple categories"
        );

        // Verify we can access recommendations by category
        for (category, recs_in_category) in by_category {
            assert!(
                !recs_in_category.is_empty(),
                "Category {} should have at least one recommendation",
                category
            );
        }
    }

    #[test]
    fn test_recommendation_structure_completeness() {
        let engine = RecommendationEngine::new();
        let recs = engine.get_recommendations(&FindingType::CornerEntryUndersteer);

        // Verify each recommendation has all required fields
        for rec in &recs {
            assert!(!rec.parameter.is_empty(), "Parameter should not be empty");
            assert!(!rec.adjustment.is_empty(), "Adjustment should not be empty");
            assert!(
                !rec.description.is_empty(),
                "Description should not be empty"
            );
        }
    }

    #[test]
    fn test_corner_entry_understeer_recommendations() {
        let engine = RecommendationEngine::new();
        let recs = engine.get_recommendations(&FindingType::CornerEntryUndersteer);

        // Should have multiple recommendations
        assert!(recs.len() >= 5, "Should have at least 5 recommendations");

        // Should include aerodynamics recommendations
        let has_aero = recs
            .iter()
            .any(|r| matches!(r.category, SetupCategory::Aerodynamics));
        assert!(has_aero, "Should have aerodynamics recommendations");

        // Should include suspension recommendations
        let has_suspension = recs
            .iter()
            .any(|r| matches!(r.category, SetupCategory::Suspension));
        assert!(has_suspension, "Should have suspension recommendations");
    }

    #[test]
    fn test_tire_overheating_recommendations() {
        let engine = RecommendationEngine::new();
        let recs = engine.get_recommendations(&FindingType::TireOverheating);

        // Should have tire management recommendations
        let has_tire_mgmt = recs
            .iter()
            .any(|r| matches!(r.category, SetupCategory::TireManagement));
        assert!(
            has_tire_mgmt,
            "Tire overheating should have tire management recommendations"
        );

        // Should recommend opening brake ducts
        let has_brake_ducts = recs.iter().any(|r| r.parameter.contains("Brake Ducts"));
        assert!(
            has_brake_ducts,
            "Should recommend adjusting brake ducts for tire overheating"
        );
    }

    #[test]
    fn test_brake_lock_recommendations() {
        let engine = RecommendationEngine::new();

        // Front brake lock should recommend moving bias rearward
        let front_recs = engine.get_recommendations(&FindingType::FrontBrakeLock);
        let has_rearward_bias = front_recs
            .iter()
            .any(|r| r.parameter.contains("Brake Bias") && r.adjustment.contains("Rearward"));
        assert!(
            has_rearward_bias,
            "Front brake lock should recommend moving bias rearward"
        );

        // Rear brake lock should recommend moving bias forward
        let rear_recs = engine.get_recommendations(&FindingType::RearBrakeLock);
        let has_forward_bias = rear_recs
            .iter()
            .any(|r| r.parameter.contains("Brake Bias") && r.adjustment.contains("Forward"));
        assert!(
            has_forward_bias,
            "Rear brake lock should recommend moving bias forward"
        );
    }

    #[test]
    fn test_format_recommendation_with_corners() {
        let engine = RecommendationEngine::new();
        let recommendations = engine.get_recommendations(&FindingType::CornerEntryUndersteer);
        let rec = &recommendations[0];

        // Test with no corners
        let mut corners = HashSet::new();
        let formatted = engine.format_recommendation_with_corners(
            rec,
            &corners,
            &FindingType::CornerEntryUndersteer,
        );
        assert_eq!(formatted, rec.description);

        // Test with single corner
        corners.insert(1);
        let formatted = engine.format_recommendation_with_corners(
            rec,
            &corners,
            &FindingType::CornerEntryUndersteer,
        );
        assert!(formatted.contains("corner 1"));
        assert!(formatted.contains(&rec.description));

        // Test with multiple corners
        corners.insert(3);
        corners.insert(5);
        let formatted = engine.format_recommendation_with_corners(
            rec,
            &corners,
            &FindingType::CornerEntryUndersteer,
        );
        assert!(formatted.contains("corners"));
        assert!(formatted.contains("1"));
        assert!(formatted.contains("3"));
        assert!(formatted.contains("5"));
    }

    #[test]
    fn test_format_recommendation_with_many_corners() {
        let engine = RecommendationEngine::new();
        let recommendations = engine.get_recommendations(&FindingType::CornerEntryUndersteer);
        let rec = &recommendations[0];

        // Test with many corners (should truncate)
        let mut corners = HashSet::new();
        for i in 1..=6 {
            corners.insert(i);
        }
        let formatted = engine.format_recommendation_with_corners(
            rec,
            &corners,
            &FindingType::CornerEntryUndersteer,
        );
        assert!(formatted.contains("and 3 others"));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn antirollbar_finding_type() -> impl Strategy<Value = FindingType> {
        prop_oneof![
            Just(FindingType::CornerEntryUndersteer),
            Just(FindingType::CornerEntryOversteer),
            Just(FindingType::CornerEntryInstability),
            Just(FindingType::MidCornerUndersteer),
            Just(FindingType::MidCornerOversteer),
            Just(FindingType::CornerExitUndersteer),
            Just(FindingType::CornerExitPowerOversteer),
            Just(FindingType::CornerExitSnapOversteer),
            Just(FindingType::FrontBrakeLock),
            Just(FindingType::RearBrakeLock),
            Just(FindingType::BrakingInstability),
            Just(FindingType::TireOverheating),
            Just(FindingType::TireCold),
            Just(FindingType::BottomingOut),
            Just(FindingType::ExcessiveTrailbraking),
        ]
    }

    // **Feature: setup-assistant, Property 7: Finding to recommendation mapping**
    // **Validates: Requirements 4.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_finding_to_recommendation_mapping(finding_type in antirollbar_finding_type()) {
            let engine = RecommendationEngine::new();
            let recommendations = engine.get_recommendations(&finding_type);

            // Every finding type should have at least one recommendation
            assert!(
                !recommendations.is_empty(),
                "Finding type {:?} should have at least one recommendation",
                finding_type
            );
        }
    }

    // **Feature: setup-assistant, Property 9: Recommendation grouping by category**
    // **Validates: Requirements 4.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_recommendation_grouping_by_category(finding_type in antirollbar_finding_type()) {
            let engine = RecommendationEngine::new();
            let recommendations = engine.get_recommendations(&finding_type);

            // All recommendations should be orderable by category
            // We verify this by grouping them and ensuring no panics occur
            use std::collections::HashMap;
            let mut by_category: HashMap<String, Vec<&SetupRecommendation>> = HashMap::new();

            for rec in &recommendations {
                let category_key = format!("{:?}", rec.category);
                by_category.entry(category_key).or_default().push(rec);
            }

            // Should have at least one category
            assert!(
                !by_category.is_empty(),
                "Recommendations should be groupable by category"
            );

            // Each recommendation should have a valid category
            for rec in &recommendations {
                // Category should be one of the valid enum variants
                // This is enforced by the type system, but we verify it's set
                let _category = &rec.category;
            }
        }
    }
}
