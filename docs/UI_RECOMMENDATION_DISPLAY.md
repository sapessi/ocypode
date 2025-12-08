# Setup Recommendations UI Display

## Overview

The Setup Window now displays prioritized recommendations with conflict detection, helping drivers focus on the most impactful changes first.

## Visual Layout

### Header Section
```
Setup Recommendations
Sorted by priority • ⚠️ = Conflicting recommendations
```

### Recommendation Display Format

Each recommendation shows:
1. **Priority Badge** (P1-P5) - Color-coded by importance
2. **Conflict Indicator** (⚠️) - Shows when recommendations conflict
3. **Parameter Name** - What to adjust (in orange)
4. **Adjustment Direction** - How to adjust it
5. **Description** - Why this helps (gray, italics)
6. **Conflict Details** - Lists conflicting recommendations (when present)

## Example Display

```
Setup Recommendations
Sorted by priority • ⚠️ = Conflicting recommendations

Brakes
  P5 ⚠️ Brake Bias - Move Forward
     Moving brake bias forward increases rear stability under braking
     ⚠️ Conflicts with: Brake Bias (Move Rearward)

  P5 • Brake Ducts - Open
     Opening brake ducts increases cooling to tires

Antirollbar
  P5 ⚠️ Front Antirollbar - Soften
     Softer front anti-roll bar allows more front grip during corner entry
     ⚠️ Conflicts with: Front Antirollbar (Stiffen)

Suspension
  P4 ⚠️ Front Springs - Soften
     Softer front springs improve mechanical grip during turn-in
     ⚠️ Conflicts with: Front Springs (Stiffen)

  P4 • Rear Springs - Soften
     Softer rear springs improve rear mechanical grip

Aero
  P3 • Rear Ride Height - Reduce
     Lowering rear ride height increases rear downforce and stability

Dampers
  P2 • Front Bump - Soften
     Softer front bump damping allows weight transfer to front tires
```

## Priority Color Coding

- **P5** (Red): Highest priority - Easy to adjust, high impact
- **P4** (Orange): High priority - Significant impact
- **P3** (Yellow): Medium priority - Moderate impact
- **P2** (Light Green): Lower priority - Fine-tuning
- **P1** (Gray): Lowest priority - Specialized adjustments

## Conflict Indicators

### When Conflicts Appear
Conflicts appear when multiple confirmed findings suggest opposing adjustments to the same parameter:

**Common Conflict Scenarios:**
- **Understeer + Oversteer**: Opposing balance adjustments
- **Tire Overheating + Cold Tires**: Opposing temperature management
- **Entry Issues + Exit Issues**: Different corner phase requirements

### Interpreting Conflicts

When you see conflicts:
1. **Check occurrence counts** - Focus on the more frequent issue
2. **Consider corner phases** - Entry vs mid-corner vs exit
3. **Review severity** - Which issue costs more lap time
4. **Driver preference** - Some conflicts require choosing a handling characteristic

### Example Conflict Interpretation

```
⚠️ Brake Bias - Move Forward (for oversteer)
⚠️ Conflicts with: Brake Bias (Move Rearward) (for understeer)
```

**This means:**
- The car exhibits both understeer and oversteer
- Likely in different corner phases or conditions
- Review telemetry to see where each occurs
- Prioritize the more frequent or severe issue
- Consider if driving technique can address one issue

## Benefits

### 1. Focus on High-Impact Changes
Priority badges help drivers start with adjustments that yield the biggest improvements.

### 2. Understand Trade-offs
Conflict warnings reveal when the car has opposing characteristics, helping drivers:
- Identify inconsistent handling
- Make informed setup decisions
- Understand which issues are related

### 3. Efficient Setup Process
By sorting recommendations by priority:
- Start with P5 recommendations
- Test and evaluate
- Move to P4 if needed
- Avoid over-adjusting with low-priority changes

### 4. Learn Setup Relationships
Seeing conflicts helps drivers understand:
- How different issues relate
- Which parameters affect multiple characteristics
- Setup trade-offs and compromises

## Usage Tips

### For New Drivers
1. Focus on P5 recommendations first
2. Make one change at a time
3. Test thoroughly before moving to next priority
4. Ignore conflicts initially - focus on the most frequent issue

### For Experienced Drivers
1. Review all priorities to understand full picture
2. Use conflicts to identify setup direction
3. Consider making opposing adjustments to different axles
4. Use priority to guide testing order

### When You See Many Conflicts
This indicates:
- Inconsistent car behavior
- Multiple issues need addressing
- May need to choose a setup direction
- Consider if driving style is contributing

## Technical Details

### Priority Assignment
Priorities are assigned based on:
- **Ease of adjustment**: How quickly can it be changed
- **Impact magnitude**: How much it affects handling
- **Predictability**: How consistent the effect is
- **Risk level**: How likely to cause new issues

### Conflict Detection
The system detects conflicts by identifying opposing adjustments:
- Increase ↔ Reduce/Decrease
- Stiffen ↔ Soften
- Open ↔ Close
- Forward ↔ Rearward

### Grouping Strategy
Recommendations are:
1. Sorted globally by priority (highest first)
2. Grouped by category for organization
3. Displayed with priority preserved within categories
