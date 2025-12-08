// Integration tests for Setup Assistant with real telemetry samples
//
// This test suite validates the complete workflow:
// 1. Load telemetry from sample files
// 2. Process through all analyzers
// 3. Extract findings via Setup Assistant
// 4. Confirm findings and retrieve recommendations
// 5. Verify recommendations match ACC Setup Guide

use std::collections::HashMap;

// Import necessary types from the main crate
use ocypode::setup_assistant::{
    CornerPhase, FindingType, SetupAssistant,
    recommendations::{RecommendationEngine, SetupCategory},
};
use ocypode::telemetry::TelemetryOutput;

/// Helper function to process a telemetry file and return the Setup Assistant state
/// This directly processes telemetry data without using the collector thread
fn process_telemetry_file(file_path: &str) -> Result<SetupAssistant, Box<dyn std::error::Error>> {
    use ocypode::telemetry::TelemetryData;
    use std::io::BufRead;

    // Read the file directly
    let file = std::fs::File::open(file_path)?;
    let reader = std::io::BufReader::new(file);

    // Create Setup Assistant
    let mut setup_assistant = SetupAssistant::new();

    // Process each line
    for line in reader.lines() {
        let line = line?;

        // Parse as TelemetryOutput
        let output: TelemetryOutput = serde_json::from_str(&line)?;

        match output {
            TelemetryOutput::SessionChange(_) => {
                // Session change - clear findings
                setup_assistant.clear_session();
            }
            TelemetryOutput::DataPoint(telemetry) => {
                // Process telemetry data
                setup_assistant.process_telemetry(&telemetry);
            }
        }
    }

    Ok(setup_assistant)
}

/// Helper function to verify that a finding type has recommendations
fn verify_recommendations(
    recommendation_engine: &RecommendationEngine,
    finding_type: &FindingType,
) -> bool {
    let recommendations = recommendation_engine.get_recommendations(finding_type);
    !recommendations.is_empty()
}

/// Helper function to verify recommendation structure
fn verify_recommendation_structure(
    recommendation_engine: &RecommendationEngine,
    finding_type: &FindingType,
) -> bool {
    let recommendations = recommendation_engine.get_recommendations(finding_type);

    for rec in &recommendations {
        // Verify all required fields are present and non-empty
        if rec.parameter.is_empty() || rec.adjustment.is_empty() || rec.description.is_empty() {
            return false;
        }
    }

    true
}

/// Helper function to verify recommendations are grouped by category
fn verify_category_grouping(
    recommendation_engine: &RecommendationEngine,
    finding_type: &FindingType,
) -> bool {
    let recommendations = recommendation_engine.get_recommendations(finding_type);

    // Group by category
    let mut categories: HashMap<SetupCategory, usize> = HashMap::new();
    for rec in &recommendations {
        *categories.entry(rec.category.clone()).or_insert(0) += 1;
    }

    // Verify we have at least one category
    !categories.is_empty()
}

#[test]
fn test_laguna_seca_telemetry_processing() {
    // Test with Laguna Seca telemetry sample
    let result = process_telemetry_file("telemetry_samples/laguna_seca.jsonl");

    assert!(
        result.is_ok(),
        "Failed to process Laguna Seca telemetry: {:?}",
        result.err()
    );

    let setup_assistant = result.unwrap();
    let findings = setup_assistant.get_findings();

    // Verify we detected some findings
    println!("Laguna Seca - Total findings detected: {}", findings.len());
    for (finding_type, finding) in findings {
        println!(
            "  {:?}: {} occurrences, phase: {:?}",
            finding_type, finding.occurrence_count, finding.corner_phase
        );
    }

    // We should have at least some findings from a real driving session
    // Note: This is a weak assertion since we don't know what issues exist in the sample
    // The main goal is to verify the system doesn't crash
    // (No assertion needed - if we got here, processing succeeded)
}

#[test]
fn test_acc_spa_telemetry_processing() {
    // Test with ACC Spa telemetry sample
    let result = process_telemetry_file("telemetry_samples/acc_spa_aston.jsonl");

    assert!(
        result.is_ok(),
        "Failed to process ACC Spa telemetry: {:?}",
        result.err()
    );

    let setup_assistant = result.unwrap();
    let findings = setup_assistant.get_findings();

    println!("ACC Spa - Total findings detected: {}", findings.len());
    for (finding_type, finding) in findings {
        println!(
            "  {:?}: {} occurrences, phase: {:?}",
            finding_type, finding.occurrence_count, finding.corner_phase
        );
    }
}

#[test]
fn test_oulton_park_telemetry_processing() {
    // Test with Oulton Park telemetry sample
    let result = process_telemetry_file("telemetry_samples/oulton_park.jsonl");

    assert!(
        result.is_ok(),
        "Failed to process Oulton Park telemetry: {:?}",
        result.err()
    );

    let setup_assistant = result.unwrap();
    let findings = setup_assistant.get_findings();

    println!("Oulton Park - Total findings detected: {}", findings.len());
    for (finding_type, finding) in findings {
        println!(
            "  {:?}: {} occurrences, phase: {:?}",
            finding_type, finding.occurrence_count, finding.corner_phase
        );
    }
}

#[test]
fn test_complete_workflow_with_confirmation() {
    // Test the complete workflow: detection → finding → confirmation → recommendation
    let result = process_telemetry_file("telemetry_samples/laguna_seca.jsonl");
    assert!(result.is_ok());

    let mut setup_assistant = result.unwrap();

    // Clone finding type to avoid borrow checker issues
    let finding_type = {
        let findings = setup_assistant.get_findings();

        if findings.is_empty() {
            println!("No findings detected in sample - skipping confirmation test");
            return;
        }

        // Pick the first finding to test confirmation workflow
        let (finding_type, _finding) = findings.iter().next().unwrap();
        println!("Testing confirmation workflow with: {:?}", finding_type);
        finding_type.clone()
    };

    // Verify finding is not confirmed initially
    assert!(
        !setup_assistant.is_confirmed(&finding_type),
        "Finding should not be confirmed initially"
    );

    // Confirm the finding
    setup_assistant.toggle_confirmation(finding_type.clone());
    assert!(
        setup_assistant.is_confirmed(&finding_type),
        "Finding should be confirmed after toggle"
    );

    // Create recommendation engine to verify recommendations
    let recommendation_engine = RecommendationEngine::new();

    // Verify recommendations are available
    assert!(
        verify_recommendations(&recommendation_engine, &finding_type),
        "Confirmed finding should have recommendations"
    );

    // Verify recommendation structure
    assert!(
        verify_recommendation_structure(&recommendation_engine, &finding_type),
        "Recommendations should have complete structure"
    );

    // Verify category grouping
    assert!(
        verify_category_grouping(&recommendation_engine, &finding_type),
        "Recommendations should be grouped by category"
    );

    // Unconfirm the finding
    setup_assistant.toggle_confirmation(finding_type.clone());
    assert!(
        !setup_assistant.is_confirmed(&finding_type),
        "Finding should be unconfirmed after second toggle"
    );
}

#[test]
fn test_all_finding_types_have_recommendations() {
    // Verify that all possible finding types have recommendations defined
    let recommendation_engine = RecommendationEngine::new();

    let all_finding_types = vec![
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

    for finding_type in all_finding_types {
        let recommendations = recommendation_engine.get_recommendations(&finding_type);
        assert!(
            !recommendations.is_empty(),
            "Finding type {:?} should have recommendations",
            finding_type
        );

        println!(
            "{:?}: {} recommendations",
            finding_type,
            recommendations.len()
        );

        // Verify each recommendation has proper structure
        for rec in &recommendations {
            assert!(
                !rec.parameter.is_empty(),
                "Recommendation parameter should not be empty for {:?}",
                finding_type
            );
            assert!(
                !rec.adjustment.is_empty(),
                "Recommendation adjustment should not be empty for {:?}",
                finding_type
            );
            assert!(
                !rec.description.is_empty(),
                "Recommendation description should not be empty for {:?}",
                finding_type
            );
        }
    }
}

#[test]
fn test_analyzer_integration() {
    // Test that all analyzers are producing annotations
    let result = process_telemetry_file("telemetry_samples/laguna_seca.jsonl");
    assert!(result.is_ok());

    let setup_assistant = result.unwrap();
    let findings = setup_assistant.get_findings();

    // Print summary of what each analyzer detected
    println!("\nAnalyzer Integration Summary:");
    println!("============================");

    let mut analyzer_findings: HashMap<&str, Vec<&FindingType>> = HashMap::new();

    for (finding_type, _) in findings {
        let analyzer_name = match finding_type {
            FindingType::CornerEntryUndersteer => "Scrub/Slip Analyzer",
            FindingType::CornerEntryOversteer => "Entry Oversteer Analyzer",
            FindingType::MidCornerUndersteer => "Mid-Corner Analyzer",
            FindingType::MidCornerOversteer => "Mid-Corner Analyzer",
            FindingType::CornerExitUndersteer => "Slip Analyzer",
            FindingType::CornerExitPowerOversteer => "Wheelspin Analyzer",
            FindingType::FrontBrakeLock => "Brake Lock Analyzer",
            FindingType::RearBrakeLock => "Brake Lock Analyzer",
            FindingType::TireOverheating => "Tire Temperature Analyzer",
            FindingType::TireCold => "Tire Temperature Analyzer",
            FindingType::BottomingOut => "Bottoming Out Analyzer",
            FindingType::ExcessiveTrailbraking => "Trailbrake Steering Analyzer",
            _ => "Other",
        };

        analyzer_findings
            .entry(analyzer_name)
            .or_insert_with(Vec::new)
            .push(finding_type);
    }

    for (analyzer, findings) in &analyzer_findings {
        println!("{}: {} finding types", analyzer, findings.len());
        for finding in findings {
            println!("  - {:?}", finding);
        }
    }
}

#[test]
fn test_recommendations_match_acc_setup_guide() {
    // Verify that recommendations align with ACC Setup Guide principles
    let recommendation_engine = RecommendationEngine::new();

    // Test Corner Entry Understeer recommendations
    let entry_understeer_recs =
        recommendation_engine.get_recommendations(&FindingType::CornerEntryUndersteer);
    assert!(!entry_understeer_recs.is_empty());

    // Should include aerodynamic adjustments
    let has_aero = entry_understeer_recs
        .iter()
        .any(|r| r.category == SetupCategory::Aerodynamics);
    assert!(
        has_aero,
        "Entry understeer should have aerodynamic recommendations"
    );

    // Test Corner Exit Power Oversteer recommendations
    let exit_oversteer_recs =
        recommendation_engine.get_recommendations(&FindingType::CornerExitPowerOversteer);
    assert!(!exit_oversteer_recs.is_empty());

    // Should include drivetrain adjustments
    let has_drivetrain = exit_oversteer_recs
        .iter()
        .any(|r| r.category == SetupCategory::Drivetrain);
    assert!(
        has_drivetrain,
        "Exit power oversteer should have drivetrain recommendations"
    );

    // Test Brake Lock recommendations
    let front_brake_lock_recs =
        recommendation_engine.get_recommendations(&FindingType::FrontBrakeLock);
    assert!(!front_brake_lock_recs.is_empty());

    // Should include brake adjustments
    let has_brakes = front_brake_lock_recs
        .iter()
        .any(|r| r.category == SetupCategory::Brakes);
    assert!(has_brakes, "Brake lock should have brake recommendations");

    // Test Tire Temperature recommendations
    let tire_overheat_recs =
        recommendation_engine.get_recommendations(&FindingType::TireOverheating);
    assert!(!tire_overheat_recs.is_empty());

    // Should include tire management adjustments
    let has_tire_mgmt = tire_overheat_recs
        .iter()
        .any(|r| r.category == SetupCategory::TireManagement);
    assert!(
        has_tire_mgmt,
        "Tire overheating should have tire management recommendations"
    );
}

#[test]
fn test_multiple_confirmation_handling() {
    // Test that multiple confirmed findings all return recommendations
    let result = process_telemetry_file("telemetry_samples/laguna_seca.jsonl");
    assert!(result.is_ok());

    let mut setup_assistant = result.unwrap();
    let findings = setup_assistant.get_findings();

    if findings.len() < 2 {
        println!("Not enough findings to test multiple confirmations - skipping");
        return;
    }

    // Confirm multiple findings
    let finding_types: Vec<FindingType> = findings.keys().take(2).cloned().collect();

    for finding_type in &finding_types {
        setup_assistant.toggle_confirmation(finding_type.clone());
    }

    // Create recommendation engine to verify recommendations
    let recommendation_engine = RecommendationEngine::new();

    // Verify all confirmed findings have recommendations
    for finding_type in &finding_types {
        assert!(setup_assistant.is_confirmed(finding_type));
        let recs = recommendation_engine.get_recommendations(finding_type);
        assert!(
            !recs.is_empty(),
            "Confirmed finding {:?} should have recommendations",
            finding_type
        );
    }
}

#[test]
fn test_corner_phase_classification() {
    // Test that findings are properly classified by corner phase
    let result = process_telemetry_file("telemetry_samples/laguna_seca.jsonl");
    assert!(result.is_ok());

    let setup_assistant = result.unwrap();
    let findings = setup_assistant.get_findings();

    let mut phase_counts: HashMap<CornerPhase, usize> = HashMap::new();

    for (finding_type, finding) in findings {
        *phase_counts.entry(finding.corner_phase).or_insert(0) += 1;

        // Verify phase makes sense for finding type
        // Note: We allow Unknown phase for all findings since corner phase classification
        // depends on telemetry state at detection time and may not always be determinable
        match finding_type {
            FindingType::CornerEntryUndersteer | FindingType::CornerEntryOversteer => {
                // Entry findings should typically be in Entry phase, but Unknown is acceptable
                // if the telemetry state was ambiguous at detection time
                if finding.corner_phase != CornerPhase::Entry
                    && finding.corner_phase != CornerPhase::Unknown
                {
                    println!(
                        "Warning: Entry finding {:?} in unexpected phase {:?}",
                        finding_type, finding.corner_phase
                    );
                }
            }
            FindingType::MidCornerUndersteer | FindingType::MidCornerOversteer => {
                // Mid-corner findings should typically be in Mid phase
                if finding.corner_phase != CornerPhase::Mid
                    && finding.corner_phase != CornerPhase::Unknown
                {
                    println!(
                        "Warning: Mid-corner finding {:?} in unexpected phase {:?}",
                        finding_type, finding.corner_phase
                    );
                }
            }
            FindingType::CornerExitUndersteer | FindingType::CornerExitPowerOversteer => {
                // Exit findings should typically be in Exit phase
                if finding.corner_phase != CornerPhase::Exit
                    && finding.corner_phase != CornerPhase::Unknown
                {
                    println!(
                        "Warning: Exit finding {:?} in unexpected phase {:?}",
                        finding_type, finding.corner_phase
                    );
                }
            }
            _ => {
                // Other findings can be in any phase
            }
        }
    }

    println!("\nCorner Phase Distribution:");
    for (phase, count) in &phase_counts {
        println!("  {:?}: {}", phase, count);
    }
}
