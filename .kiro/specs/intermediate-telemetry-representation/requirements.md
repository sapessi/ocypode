# Requirements Document

## Introduction

This document specifies the requirements for creating an intermediate telemetry representation layer in Ocypode. Currently, the application relies on the simetry library's `Moment` trait, which provides a limited set of telemetry data points through its base interface. Game-specific implementations (iRacing and Assetto Corsa Competizione) expose additional data fields that are not accessible through the base trait, requiring unsafe downcasting and game-specific extraction logic.

The intermediate representation will decouple the telemetry analyzers from game-specific implementations, allowing analyzers to work with a unified data structure that captures all possible data points from both iRacing and ACC. This will eliminate the need for unsafe code, simplify analyzer implementations, and make it easier to add support for additional racing simulations in the future.

## Glossary

- **Telemetry System**: The Ocypode application component responsible for collecting, processing, and displaying racing simulation data
- **Intermediate Representation**: A unified data structure that captures all telemetry data points from multiple racing simulations
- **Producer**: A component that connects to a racing simulation and retrieves raw telemetry data
- **Analyzer**: A component that processes telemetry data to detect driving issues (slip, wheelspin, scrubbing, etc.)
- **simetry**: An external Rust library that provides a unified interface for accessing telemetry from multiple racing simulations
- **Moment**: A trait from the simetry library representing a single point of telemetry data
- **iRacing**: A racing simulation game supported by Ocypode
- **ACC**: Assetto Corsa Competizione, a racing simulation game supported by Ocypode
- **Game-specific Producer**: A producer implementation that connects to a specific racing simulation (e.g., IRacingTelemetryProducer, ACCTelemetryProducer)

## Requirements

### Requirement 1

**User Story:** As a developer, I want a unified intermediate telemetry representation, so that analyzers can access all available data points without knowing which game the data came from.

#### Acceptance Criteria

1. WHEN a producer retrieves telemetry from any supported game THEN the Telemetry System SHALL convert it to the intermediate representation
2. WHEN an analyzer processes telemetry data THEN the Telemetry System SHALL provide the intermediate representation instead of the raw Moment trait object
3. WHEN the intermediate representation is created THEN the Telemetry System SHALL preserve all data fields available from the source game
4. WHERE a data field is not available from a particular game THEN the intermediate representation SHALL represent it as None
5. WHEN converting from simetry Moment to intermediate representation THEN the Telemetry System SHALL not use unsafe code for type casting

### Requirement 2

**User Story:** As a developer, I want the intermediate representation to capture all iRacing data points, so that no telemetry information is lost during conversion.

#### Acceptance Criteria

1. WHEN converting iRacing telemetry THEN the Telemetry System SHALL extract vehicle state data including gear, speed, engine RPM, maximum engine RPM, and shift point RPM
2. WHEN converting iRacing telemetry THEN the Telemetry System SHALL extract input data including throttle, brake, clutch, steering angle, and steering percentage
3. WHEN converting iRacing telemetry THEN the Telemetry System SHALL extract position data including lap distance, lap distance percentage, and lap number
4. WHEN converting iRacing telemetry THEN the Telemetry System SHALL extract timing data including last lap time and best lap time
5. WHEN converting iRacing telemetry THEN the Telemetry System SHALL extract flag data including pit limiter status, pit lane status, and ABS active status
6. WHEN converting iRacing telemetry THEN the Telemetry System SHALL extract GPS coordinates including latitude and longitude
7. WHEN converting iRacing telemetry THEN the Telemetry System SHALL extract acceleration data including lateral and longitudinal acceleration
8. WHEN converting iRacing telemetry THEN the Telemetry System SHALL extract orientation data including pitch, pitch rate, roll, roll rate, yaw, and yaw rate
9. WHEN converting iRacing telemetry THEN the Telemetry System SHALL extract tire temperature data for all four tires including carcass and surface temperatures

### Requirement 3

**User Story:** As a developer, I want the intermediate representation to capture all ACC data points, so that ACC-specific telemetry features are fully supported.

#### Acceptance Criteria

1. WHEN converting ACC telemetry THEN the Telemetry System SHALL extract vehicle state data including gear, speed, engine RPM, maximum engine RPM, and shift point RPM
2. WHEN converting ACC telemetry THEN the Telemetry System SHALL extract input data including throttle, brake, clutch, steering angle, and steering percentage
3. WHEN converting ACC telemetry THEN the Telemetry System SHALL extract position data including lap distance percentage and lap number
4. WHEN converting ACC telemetry THEN the Telemetry System SHALL extract timing data including last lap time and best lap time
5. WHEN converting ACC telemetry THEN the Telemetry System SHALL extract flag data including pit limiter status, pit lane status, and ABS active status
6. WHEN converting ACC telemetry THEN the Telemetry System SHALL extract orientation data including pitch, roll, and yaw
7. WHEN converting ACC telemetry THEN the Telemetry System SHALL extract tire temperature data for all four tires when available

### Requirement 4

**User Story:** As a developer, I want game-specific producers to handle conversion to the intermediate representation, so that the conversion logic is encapsulated within each producer implementation.

#### Acceptance Criteria

1. WHEN a game-specific producer retrieves telemetry THEN the producer SHALL convert the raw simetry data to the intermediate representation
2. WHEN the IRacingTelemetryProducer retrieves telemetry THEN the producer SHALL use iRacing-specific extraction methods to populate all available fields
3. WHEN the ACCTelemetryProducer retrieves telemetry THEN the producer SHALL use ACC-specific extraction methods to populate all available fields
4. WHEN the MockTelemetryProducer provides test data THEN the producer SHALL convert SerializableTelemetry to the intermediate representation
5. WHEN a producer cannot extract a specific data field THEN the producer SHALL set that field to None in the intermediate representation

### Requirement 5

**User Story:** As a developer, I want analyzers to work with the intermediate representation, so that analyzer logic is independent of the underlying game implementation.

#### Acceptance Criteria

1. WHEN an analyzer processes telemetry THEN the analyzer SHALL receive the intermediate representation instead of a Moment trait object
2. WHEN the SlipAnalyzer detects slip conditions THEN the analyzer SHALL access steering angle from the intermediate representation
3. WHEN the ScrubAnalyzer detects scrubbing THEN the analyzer SHALL access yaw rate from the intermediate representation
4. WHEN the TrailbrakeSteeringAnalyzer evaluates braking THEN the analyzer SHALL access brake and steering data from the intermediate representation
5. WHEN the WheelspinAnalyzer detects wheelspin THEN the analyzer SHALL access throttle and RPM data from the intermediate representation
6. WHEN the ShortShiftingAnalyzer evaluates gear changes THEN the analyzer SHALL access gear and RPM data from the intermediate representation

### Requirement 6

**User Story:** As a developer, I want to eliminate unsafe code from telemetry conversion, so that the application is more maintainable and less prone to undefined behavior.

#### Acceptance Criteria

1. WHEN converting telemetry from any game THEN the Telemetry System SHALL not use raw pointer casting
2. WHEN converting telemetry from any game THEN the Telemetry System SHALL not use unsafe blocks for type downcasting
3. WHEN a producer returns telemetry data THEN the producer SHALL return the intermediate representation directly
4. WHEN the collector processes telemetry THEN the collector SHALL work with the intermediate representation without type casting

### Requirement 7

**User Story:** As a developer, I want the intermediate representation to be serializable, so that telemetry data can be saved to files and loaded for offline analysis.

#### Acceptance Criteria

1. WHEN telemetry data is saved to a file THEN the Telemetry System SHALL serialize the intermediate representation to JSON
2. WHEN telemetry data is loaded from a file THEN the Telemetry System SHALL deserialize JSON to the intermediate representation
3. WHEN serializing the intermediate representation THEN the Telemetry System SHALL preserve all data fields including None values
4. WHEN deserializing the intermediate representation THEN the Telemetry System SHALL handle missing fields gracefully by setting them to None

### Requirement 8

**User Story:** As a developer, I want to replace SerializableTelemetry with the intermediate representation, so that the codebase uses a single unified telemetry structure throughout.

#### Acceptance Criteria

1. WHEN the intermediate representation is implemented THEN the Telemetry System SHALL use it as the primary telemetry data structure
2. WHEN telemetry data is saved to files THEN the Telemetry System SHALL serialize the intermediate representation
3. WHEN telemetry data is loaded from files THEN the Telemetry System SHALL deserialize to the intermediate representation
4. WHEN the refactoring is complete THEN the Telemetry System SHALL remove the SerializableTelemetry struct
