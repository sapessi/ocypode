# Setup Assistant User Guide

## Overview

The Setup Assistant is a real-time telemetry analysis feature that helps you optimize your car setup in Assetto Corsa Competizione (ACC) and iRacing. It automatically detects handling issues during your driving sessions and provides specific setup recommendations based on proven setup methodology.

## How It Works

The Setup Assistant operates in three stages:

1. **Detection**: Analyzes your telemetry data in real-time to identify handling issues
2. **Confirmation**: You review detected issues and confirm the ones you actually feel in the car
3. **Recommendations**: Provides specific setup changes to address your confirmed issues

## Opening the Setup Window

To access the Setup Assistant:

1. Start Ocypode in live mode with your racing simulation
2. Look for the Setup Assistant button in the main interface (positioned near the alerts window control)
3. Click the button to toggle the Setup Window visibility

The Setup Window will remember its position and visibility state between sessions.

## Using the Setup Assistant

### Step 1: Drive and Collect Data

Simply drive as you normally would. The Setup Assistant continuously monitors your telemetry and detects potential handling issues:

- **Corner Entry Issues**: Understeer, oversteer, instability during braking and turn-in
- **Mid-Corner Issues**: Understeer or oversteer during the apex phase
- **Corner Exit Issues**: Understeer, power oversteer, wheelspin
- **Braking Issues**: Front or rear brake locking
- **Tire Issues**: Overheating or cold tires
- **Suspension Issues**: Bottoming out over bumps

### Step 2: Review Detected Issues

The Setup Window displays all detected issues with:
- **Issue Type**: What kind of handling problem was detected
- **Occurrence Count**: How many times it was detected (updates in real-time)
- **Corner Phase**: Where in the corner it typically occurs (Entry, Mid, Exit, Straight)

Example:
```
Corner Entry Understeer (12) - Entry
Mid-Corner Oversteer (5) - Mid-Corner
Tire Overheating (8) - Unknown
```

If no issues are detected, you'll see:
```
No issues detected
Drive a few laps to collect data
```

### Step 3: Confirm Issues You Feel

**Important**: Not every detected issue is necessarily a problem you want to fix. The Setup Assistant detects potential issues, but you should only confirm the ones you actually feel in the car.

To confirm an issue:
1. Click on the issue in the list
2. The issue will be highlighted (filled button style)
3. Setup recommendations will appear below

To unconfirm an issue:
1. Click on the confirmed issue again
2. The issue will return to normal style
3. Its recommendations will be hidden

You can confirm multiple issues simultaneously - the Setup Assistant will show recommendations for all confirmed issues.

### Step 4: Apply Recommendations

Once you've confirmed one or more issues, the Setup Window displays specific setup recommendations grouped by category:

**Categories**:
- **Aero**: Wing angles, ride height, splitter
- **Suspension**: Spring rates, ride height
- **ARB**: Anti-roll bar stiffness
- **Dampers**: Bump and rebound settings
- **Brakes**: Brake bias and pressure
- **Drivetrain**: Differential settings
- **Electronics**: Traction control, ABS
- **Alignment**: Camber, toe, caster
- **Tire Mgmt**: Brake ducts, tire pressure

Each recommendation shows:
- **Parameter**: What to adjust (e.g., "Front Ride Height")
- **Adjustment**: Direction of change (e.g., "Reduce", "Increase", "Soften")
- **Description**: Why this adjustment helps

Example:
```
Aero
• Front Ride Height - Reduce
  Lowering front ride height increases front downforce and grip
• Rear Ride Height - Increase
  Raising rear ride height reduces rear downforce, shifting balance forward

Suspension
• Front Springs - Soften
  Softer front springs improve mechanical grip during turn-in
```

### Step 5: Make Setup Changes

1. Exit to the garage/pits
2. Open your car's setup menu
3. Apply the recommended changes one at a time or in combination
4. Test the changes on track
5. Repeat the process to fine-tune

**Tip**: Start with one or two changes at a time so you can feel their individual effects.

## Understanding Detected Issues

### Corner Entry Issues

**Corner Entry Understeer**
- **What it is**: Front tires lose grip during braking and turn-in, car won't turn
- **How it's detected**: Front tire scrubbing or slip during braking with steering input
- **Common causes**: Too much front downforce, stiff front suspension, forward brake bias

**Corner Entry Oversteer**
- **What it is**: Rear slides out during braking and turn-in
- **How it's detected**: Yaw rate exceeds expected response during braking with steering
- **Common causes**: Too much rear downforce, stiff rear suspension, rearward brake bias

### Mid-Corner Issues

**Mid-Corner Understeer**
- **What it is**: Front loses grip at apex while coasting
- **How it's detected**: Speed loss during coasting with steering input
- **Common causes**: Insufficient front downforce, stiff front ARB

**Mid-Corner Oversteer**
- **What it is**: Rear becomes unstable at apex
- **How it's detected**: Excessive yaw rate during coasting with steering
- **Common causes**: Insufficient rear downforce, soft rear ARB

### Corner Exit Issues

**Corner Exit Understeer**
- **What it is**: Front loses grip when applying throttle
- **How it's detected**: Front tire slip during throttle application
- **Common causes**: Insufficient differential locking, soft front suspension

**Corner Exit Power Oversteer**
- **What it is**: Rear wheelspin when applying throttle
- **How it's detected**: Rear wheel speed exceeds expected for acceleration
- **Common causes**: Too much differential locking, insufficient traction control

### Braking Issues

**Front Brake Lock**
- **What it is**: Front wheels lock under braking (ABS activating)
- **How it's detected**: ABS activation with higher front tire slip
- **Common causes**: Too much brake bias forward, excessive brake pressure

**Rear Brake Lock**
- **What it is**: Rear wheels lock under braking (ABS activating)
- **How it's detected**: ABS activation with higher rear tire slip
- **Common causes**: Too much brake bias rearward

### Tire Issues

**Tire Overheating**
- **What it is**: Tire temperatures consistently above optimal range (95°C+)
- **How it's detected**: Average tire temperature sustained above 95°C
- **Common causes**: Closed brake ducts, stiff suspension, aggressive driving

**Cold Tires**
- **What it is**: Tire temperatures consistently below optimal range (80°C-)
- **How it's detected**: Average tire temperature sustained below 80°C
- **Common causes**: Open brake ducts, soft suspension, not enough load

### Suspension Issues

**Bottoming Out**
- **What it is**: Suspension compresses fully, hitting bump stops
- **How it's detected**: Sudden pitch change with speed loss on straights or over bumps
- **Common causes**: Too low ride height, soft springs, soft bump damping

## Tips for Best Results

### Data Collection

1. **Drive multiple laps**: The more data collected, the more accurate the detection
2. **Drive consistently**: Try to drive at your normal pace, not experimenting
3. **Complete full laps**: Issues are detected throughout the lap, not just in specific corners
4. **Use representative conditions**: Test in conditions similar to your race (fuel load, tire wear)

### Confirming Issues

1. **Trust your feel**: Only confirm issues you actually experience in the car
2. **Consider frequency**: Higher occurrence counts suggest more significant issues
3. **Check corner phase**: Ensure the detected phase matches where you feel the issue
4. **Be selective**: You don't need to fix every detected issue

### Applying Recommendations

1. **Start conservative**: Make small adjustments first
2. **Change one thing at a time**: This helps you understand each change's effect
3. **Test after each change**: Drive a few laps to feel the difference
4. **Document your changes**: Keep notes on what worked and what didn't
5. **Iterate**: Setup is an iterative process - repeat the cycle

### Session Management

- **New session = fresh start**: When you start a new session, all findings are cleared
- **Window state persists**: Your window position and confirmed findings are saved when you close the window
- **Real-time updates**: Occurrence counts update as you drive, no need to refresh

## Analyzer Configuration

The Setup Assistant uses several analyzers with specific thresholds:

### Entry Oversteer Analyzer
- **Minimum brake**: 30% brake application required
- **Minimum steering**: 10% steering input required
- **Oversteer threshold**: Yaw rate must exceed expected by 1.5x
- **Window size**: 10 samples for baseline calculation

### Mid-Corner Analyzer
- **Maximum throttle**: 15% (coasting detection)
- **Maximum brake**: 15% (coasting detection)
- **Minimum steering**: 10% steering input required
- **Understeer threshold**: 0.5 m/s speed loss
- **Oversteer threshold**: Yaw rate must exceed expected by 1.5x

### Brake Lock Analyzer
- **Minimum brake**: 30% brake application required
- **Detection**: ABS activation during braking zone

### Tire Temperature Analyzer
- **Optimal range**: 80°C - 95°C
- **History window**: 60 seconds
- **Minimum samples**: 10 samples before detection
- **Sample rate**: 1 sample per second

### Bottoming Out Analyzer
- **Minimum pitch change**: 0.05 radians
- **Minimum speed loss**: 0.5 m/s
- **Maximum steering**: 20% (filters for straights/bumps)

## Troubleshooting

### "No issues detected" but I'm having problems

**Possible causes**:
1. **Not enough data**: Drive more laps to collect sufficient telemetry
2. **Issue not covered**: The analyzer might not detect your specific issue yet
3. **Below threshold**: The issue might be below detection thresholds
4. **Missing telemetry**: Some games don't provide all telemetry fields

**Solutions**:
- Drive at least 5-10 laps for good data collection
- Check that your game is providing full telemetry data
- Consider manual setup adjustments based on your feel

### Issues detected that I don't feel

**This is normal!** The analyzers detect potential issues, but not all are problems you need to fix.

**What to do**:
- Only confirm issues you actually feel in the car
- Higher occurrence counts are more likely to be real issues
- Some detected issues might be your driving style, not setup problems

### Recommendations seem contradictory

**This can happen when you confirm multiple issues that require opposite adjustments.**

**What to do**:
- Prioritize the most frequent or most problematic issue
- Address one issue at a time
- Some issues might be related - fixing one might fix others

### Window position is off-screen

**If you move the window off-screen or change monitor setup:**

1. Close Ocypode
2. Delete or edit the config file (location varies by OS)
3. Restart Ocypode - window will appear at default position

## Advanced Usage

### Understanding Occurrence Counts

Occurrence counts help you prioritize issues:
- **1-5 occurrences**: Might be isolated incidents or specific corners
- **5-15 occurrences**: Consistent issue worth addressing
- **15+ occurrences**: Significant problem affecting multiple corners

### Combining Recommendations

Some recommendations work well together:
- **Understeer package**: Reduce front ride height + soften front springs + soften front ARB
- **Oversteer package**: Increase rear wing + soften rear springs + soften rear ARB
- **Brake balance**: Adjust bias + adjust pressure together

### Track-Specific Considerations

Different tracks emphasize different aspects:
- **High-speed tracks**: Aero adjustments more important
- **Technical tracks**: Mechanical grip (springs, ARBs) more important
- **Bumpy tracks**: Damper and ride height adjustments critical

## Frequently Asked Questions

**Q: Does the Setup Assistant work with all games?**
A: Yes, it works with any game supported by Ocypode (currently iRacing and ACC). However, some analyzers require specific telemetry fields that might not be available in all games.

**Q: Will this make me faster?**
A: The Setup Assistant helps you optimize your car setup, which can improve lap times. However, driving skill is still the most important factor. Use this tool to eliminate setup-related issues so you can focus on improving your driving.

**Q: Can I use this during a race?**
A: Yes, but it's designed for practice and qualifying sessions. During a race, focus on driving - review the findings after the session.

**Q: How accurate are the recommendations?**
A: The recommendations are based on established setup methodology from the ACC Setup Guide and general racing setup principles. They provide a good starting point, but you should always test and adjust based on your specific car, track, and driving style.

**Q: Do I need to apply all recommendations?**
A: No! Start with one or two changes and test. Setup is personal - what works for one driver might not work for another.

**Q: Can I save my setup changes?**
A: The Setup Assistant doesn't directly save setup files. You'll need to save your setup through your racing simulation's setup menu.

**Q: What if I make things worse?**
A: Keep your original setup saved! You can always revert. Make small changes and test each one so you know what helped and what didn't.

## Support and Feedback

The Setup Assistant is part of the open-source Ocypode project. If you encounter issues or have suggestions:

1. Check the GitHub repository for known issues
2. Report bugs with detailed information (game, issue type, telemetry data if possible)
3. Suggest new features or improvements
4. Contribute code if you're a developer!

## Version History

**Current Version**: Includes detection for:
- Corner entry understeer and oversteer
- Mid-corner understeer and oversteer  
- Corner exit understeer and power oversteer
- Front and rear brake locking
- Tire overheating and cold tires
- Suspension bottoming out
- Excessive trail braking

Future versions may include additional analyzers and enhanced recommendations.
