# Recommendation Prioritization and Conflict Detection

## Overview

The Setup Assistant now includes intelligent recommendation processing that prioritizes setup changes by impact and detects conflicting recommendations when multiple findings are confirmed.

## Features

### 1. Priority-Based Sorting

All recommendations are assigned a priority level (1-5) based on:
- **Priority 5**: Highest impact, easiest to adjust (ARBs, brake bias, electronics)
- **Priority 4**: High impact, moderate complexity (springs, differential)
- **Priority 3**: Medium impact (aero, alignment)
- **Priority 2**: Lower impact or more complex (dampers, fine-tuning)
- **Priority 1**: Specialized or situational adjustments

Recommendations are automatically sorted with highest priority first, helping drivers focus on the most impactful changes.

### 2. Conflict Detection

When multiple findings are confirmed (e.g., both understeer and oversteer), the system detects conflicting recommendations for the same parameter:

**Detected Conflicts:**
- Increase vs Reduce/Decrease
- Stiffen vs Soften
- Open vs Close
- Forward vs Rearward/Backward

### 3. Trade-off Identification

Conflicts highlight setup trade-offs where the car exhibits opposing characteristics. This helps drivers:
- Understand which issues are more severe
- Make informed decisions about setup direction
- Identify areas where driving style adjustments may be needed

## Usage

### Basic Usage

```rust
use ocypode::setup_assistant::{FindingType, SetupAssistant};

let mut assistant = SetupAssistant::new();

// Confirm findings
assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);
assistant.toggle_confirmation(FindingType::TireOverheating);

// Get processed recommendations
let processed = assistant.get_processed_recommendations();

for proc_rec in processed {
    let rec = &proc_rec.recommendation;
    
    println!("[Priority {}] {} - {}", 
        rec.priority,
        rec.parameter,
        rec.adjustment
    );
    
    if proc_rec.has_conflict {
        println!("  ⚠️  Conflicts with:");
        for conflict in &proc_rec.conflicts {
            println!("    - {} ({})", conflict.parameter, conflict.adjustment);
        }
    }
}
```

### ProcessedRecommendation Structure

```rust
pub struct ProcessedRecommendation {
    /// The original recommendation with priority
    pub recommendation: SetupRecommendation,
    /// List of conflicting recommendations for the same parameter
    pub conflicts: Vec<SetupRecommendation>,
    /// Whether this recommendation has conflicts
    pub has_conflict: bool,
}
```

## Priority Assignment Strategy

### High Priority (5)
- **Anti-roll bars**: Direct impact on balance, easy to adjust
- **Brake bias**: Immediate effect on braking stability
- **Traction control**: Electronic aid, no mechanical changes
- **Brake ducts**: Simple adjustment for tire temperature

### High Priority (4)
- **Springs**: Significant impact on mechanical grip
- **Differential**: Major effect on power delivery and rotation
- **Brake pressure**: Overall braking performance

### Medium Priority (3)
- **Ride height**: Affects aero and geometry
- **Wings/Splitter**: Aero balance adjustments
- **Camber**: Tire contact patch optimization

### Lower Priority (2)
- **Dampers**: Fine-tuning, more complex to dial in
- **Toe**: Affects tire wear and response
- **Bump/Rebound**: Advanced suspension tuning

## Example Scenarios

### Scenario 1: Single Finding
When only one finding is confirmed, all recommendations are conflict-free and sorted by priority.

### Scenario 2: Opposing Findings
Confirming both understeer and oversteer reveals conflicts:
- Brake Bias: Move Forward (oversteer) vs Move Rearward (understeer)
- Front Springs: Stiffen (oversteer) vs Soften (understeer)

These conflicts indicate the car has inconsistent balance, suggesting:
1. Focus on the more frequent/severe issue
2. Consider different corner phases (entry vs exit)
3. Adjust driving technique

### Scenario 3: Temperature Issues
Confirming both tire overheating and cold tires shows:
- Brake Ducts: Open (overheating) vs Close (cold)

This suggests temperature management varies by track conditions or tire compound.

## Integration with UI

The processed recommendations can be displayed in the UI with:
- Visual priority indicators (color coding, badges)
- Conflict warnings with explanations
- Grouped by category for easier navigation
- Tooltips explaining why conflicts occur

## Testing

Run the example to see the system in action:
```bash
cargo run --example recommendation_processing
```

Run the test suite:
```bash
cargo test recommendation_processing_tests
```

## Future Enhancements

Potential improvements:
- Weight conflicts by occurrence count (more frequent issue takes precedence)
- Machine learning to adjust priorities based on lap time improvements
- Driver preference profiles (aggressive vs conservative setups)
- Track-specific priority adjustments
- Telemetry-based confidence scores for recommendations
