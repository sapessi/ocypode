# Implementation Plan

- [x] 1. Create TelemetryData struct and basic infrastructure





  - Create the `TelemetryData` struct in `src/telemetry/mod.rs` with all fields as specified in the design
  - Add `Default` implementation for `TelemetryData`
  - Ensure `TelemetryData` derives `Clone`, `Debug`, `Serialize`, and `Deserialize`
  - Keep `TireInfo` struct unchanged
  - _Requirements: 1.1, 1.3, 7.1_

- [ ]* 1.1 Write unit tests for TelemetryData struct
  - Test that Default implementation creates valid TelemetryData
  - Test that all fields are properly initialized
  - _Requirements: 1.1_

- [x] 2. Implement conversion from SerializableTelemetry to TelemetryData





  - Add `TelemetryData::from_serializable()` method
  - Map all fields from SerializableTelemetry to TelemetryData
  - Handle field name changes (e.g., `lat` to `latitude_deg`)
  - _Requirements: 4.4, 8.1_

- [ ]* 2.1 Write unit tests for SerializableTelemetry conversion
  - Test conversion with all fields populated
  - Test conversion with Some and None fields
  - Test field name mappings are correct
  - _Requirements: 4.4_

- [ ]* 2.2 Write property test for SerializableTelemetry conversion
  - **Property 6: MockProducer converts SerializableTelemetry correctly**
  - **Validates: Requirements 4.4**

- [x] 3. Implement iRacing telemetry conversion





  - Add `TelemetryData::from_iracing_state()` method (Windows only)
  - Extract all fields from `simetry::iracing::SimState`
  - Use base `Moment` trait methods for common fields
  - Access iRacing-specific fields directly from the state
  - Handle unit conversions (velocity to m/s, RPM, etc.)
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9_

- [ ]* 3.1 Write unit tests for iRacing conversion
  - Test extraction of vehicle state fields
  - Test extraction of input fields
  - Test extraction of position and timing fields
  - Test extraction of orientation and acceleration fields
  - Test extraction of tire data
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9_

- [ ]* 3.2 Write property test for iRacing field extraction
  - **Property 2: iRacing telemetry extraction completeness**
  - **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9**

- [x] 4. Implement ACC telemetry conversion





  - Add `TelemetryData::from_acc_state()` method (Windows only)
  - Extract all fields from `simetry::assetto_corsa_competizione::SimState`
  - Use base `Moment` trait methods for common fields
  - Access ACC-specific fields from `state.physics` and `state.graphics`
  - Handle unit conversions
  - Set unavailable fields (GPS, rate data) to None
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7_

- [ ]* 4.1 Write unit tests for ACC conversion
  - Test extraction of vehicle state fields
  - Test extraction of input fields
  - Test extraction of position and timing fields
  - Test extraction of orientation fields
  - Test that unavailable fields are None
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7_

- [ ]* 4.2 Write property test for ACC field extraction
  - **Property 3: ACC telemetry extraction completeness**
  - **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7**

- [ ]* 4.3 Write property test for unavailable fields
  - **Property 4: Unavailable fields are represented as None**
  - **Validates: Requirements 1.4**

- [x] 5. Update TelemetryProducer trait and implementations





  - Change `TelemetryProducer::telemetry()` return type to `Result<TelemetryData, OcypodeError>`
  - Update `IRacingTelemetryProducer::telemetry()` to call `TelemetryData::from_iracing_state()`
  - Update `ACCTelemetryProducer::telemetry()` to call `TelemetryData::from_acc_state()`
  - Update `MockTelemetryProducer::telemetry()` to call `TelemetryData::from_serializable()`
  - Remove `MockMoment` struct (no longer needed)
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 6.3_

- [ ]* 5.1 Write unit tests for updated producers
  - Test that IRacingTelemetryProducer returns TelemetryData
  - Test that ACCTelemetryProducer returns TelemetryData
  - Test that MockTelemetryProducer returns TelemetryData
  - _Requirements: 4.1_

- [ ]* 5.2 Write property test for producer return type
  - **Property 5: Producers return TelemetryData**
  - **Validates: Requirements 4.1**

- [x] 6. Update TelemetryAnalyzer trait





  - Change `TelemetryAnalyzer::analyze()` parameter from `&dyn Moment` to `&TelemetryData`
  - Update trait documentation
  - _Requirements: 1.2, 5.1_

- [x] 7. Update SlipAnalyzer to use TelemetryData




  - Change `analyze()` method signature to accept `&TelemetryData`
  - Access `steering_angle_rad` from TelemetryData instead of trying to extract from Moment
  - Access `throttle`, `brake`, and `speed_mps` from TelemetryData
  - Update internal state tracking
  - _Requirements: 5.2_

- [ ]* 7.1 Write unit tests for updated SlipAnalyzer
  - Test slip detection with TelemetryData
  - Test that steering angle is correctly accessed
  - _Requirements: 5.2_

- [x] 8. Update ScrubAnalyzer to use TelemetryData





  - Change `analyze()` method signature to accept `&TelemetryData`
  - Access `yaw_rate_rps` from TelemetryData
  - Handle None values gracefully
  - _Requirements: 5.3_

- [ ]* 8.1 Write unit tests for updated ScrubAnalyzer
  - Test scrub detection with TelemetryData
  - Test that yaw rate is correctly accessed
  - Test handling of None yaw rate
  - _Requirements: 5.3_

- [x] 9. Update TrailbrakeSteeringAnalyzer to use TelemetryData





  - Change `analyze()` method signature to accept `&TelemetryData`
  - Access `brake` and `steering_angle_rad` from TelemetryData
  - _Requirements: 5.4_

- [ ]* 9.1 Write unit tests for updated TrailbrakeSteeringAnalyzer
  - Test trailbrake detection with TelemetryData
  - Test that brake and steering are correctly accessed
  - _Requirements: 5.4_

- [x] 10. Update WheelspinAnalyzer to use TelemetryData





  - Change `analyze()` method signature to accept `&TelemetryData`
  - Access `throttle` and `engine_rpm` from TelemetryData
  - _Requirements: 5.5_

- [ ]* 10.1 Write unit tests for updated WheelspinAnalyzer
  - Test wheelspin detection with TelemetryData
  - Test that throttle and RPM are correctly accessed
  - _Requirements: 5.5_

- [x] 11. Update ShortShiftingAnalyzer to use TelemetryData





  - Change `analyze()` method signature to accept `&TelemetryData`
  - Access `gear`, `engine_rpm`, and `shift_point_rpm` from TelemetryData
  - _Requirements: 5.6_

- [ ]* 11.1 Write unit tests for updated ShortShiftingAnalyzer
  - Test short shifting detection with TelemetryData
  - Test that gear and RPM data are correctly accessed
  - _Requirements: 5.6_

- [x] 12. Update telemetry collector to use TelemetryData





  - Update `collect_telemetry()` to work with TelemetryData instead of `Box<dyn Moment>`
  - Remove unsafe downcasting code
  - Pass TelemetryData to analyzers
  - Pass TelemetryData to writer and UI senders
  - _Requirements: 1.2, 6.1, 6.2, 6.4_

- [ ]* 12.1 Write unit tests for updated collector
  - Test that collector works with TelemetryData
  - Test that analyzers receive TelemetryData
  - Test that no unsafe code is used
  - _Requirements: 6.1, 6.2, 6.4_

- [x] 13. Implement serialization and deserialization





  - Verify that `Serialize` and `Deserialize` traits work correctly for TelemetryData
  - Test JSON serialization format
  - Ensure None values are preserved in JSON
  - _Requirements: 7.1, 7.2, 7.3_

- [ ]* 13.1 Write unit tests for serialization
  - Test serialization of TelemetryData with all fields
  - Test serialization with Some and None fields
  - Test that None values are preserved
  - _Requirements: 7.1, 7.3_

- [ ]* 13.2 Write property test for serialization round-trip
  - **Property 1: Serialization round-trip preserves all fields**
  - **Validates: Requirements 1.3, 7.1, 7.3**

- [ ]* 13.3 Write property test for deserialization robustness
  - **Property 7: Deserialization handles missing fields gracefully**
  - **Validates: Requirements 7.4**

- [x] 14. Update telemetry writer to use TelemetryData




  - Update writer to serialize TelemetryData instead of SerializableTelemetry
  - Update `TelemetryOutput` enum to use TelemetryData
  - _Requirements: 7.1, 8.2_

- [ ]* 14.1 Write unit tests for updated writer
  - Test that writer serializes TelemetryData correctly
  - Test file format compatibility
  - _Requirements: 7.1, 8.2_

- [x] 15. Update telemetry loader to use TelemetryData




  - Update loader to deserialize TelemetryData instead of SerializableTelemetry
  - Update MockTelemetryProducer to work with TelemetryData files
  - _Requirements: 7.2, 8.3_

- [ ]* 15.1 Write unit tests for updated loader
  - Test that loader deserializes TelemetryData correctly
  - Test loading files with missing fields
  - _Requirements: 7.2, 7.4_

- [ ] 16. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 17. Remove SerializableTelemetry and cleanup





  - Remove `SerializableTelemetry` struct from codebase
  - Remove `from_moment()`, `from_moment_boxed()`, and related unsafe code
  - Remove `extract_iracing_fields()` and `extract_acc_fields()` methods
  - Update all references to use TelemetryData
  - _Requirements: 6.1, 6.2, 8.4_

- [ ]* 17.1 Verify no unsafe code remains
  - Search codebase for `unsafe` blocks in telemetry module
  - Verify no raw pointer casting exists
  - _Requirements: 6.1, 6.2_

- [x] 18. Update documentation and examples





  - Update README.md with new telemetry file format
  - Update TELEMETRY_FILE_FORMAT.md to describe TelemetryData structure
  - Update code comments and documentation
  - Add migration notes for breaking changes
  - _Requirements: 8.1_

- [x] 19. Final checkpoint - Ensure all tests pass







  - Ensure all tests pass, ask the user if questions arise.
