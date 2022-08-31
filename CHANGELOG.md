# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.9] Q3 2022
- fixed report azure-sdk-version in twin

## [0.1.8] Q3 2022
 - fixed bug when report azure-sdk-version in twin to early when used as edge module

## [0.1.7] Q3 2022
 - report azure-sdk-version in twin
 - log info message for azure-sdk-version
 - bump to azure-iot-sdk 0.8.4

## [0.1.6] Q3 2022
 - start service after time-sync target to avoid time jumps during service start
 - added info message for logging the package version

## [0.1.5] Q3 2022
- updated all depencies to solve audit errors
- don't depend on chrono's default-features
- adapted Cargo.audit.ignore

## [0.1.4] Q3 2022
- fixed panic when receiving unauthenticated message

## [0.1.3] Q3 2022
- fixed panic when calling IotHubClient::from_identity_service
- fixed terminating on ExpiredSasToken
- bumped to latest azure-iot-sdk 0.8.3

## [0.1.2] Q3 2022
- fixed bug on initial location creation

## [0.1.1] Q3 2022
- create location only once and save as reported property
- ignored RUSTSEC-2020-0071
- removed unused dependency to notify
- bumped to latest azure-iot-sdk 0.8.2

## [0.1.0] Q3 2022
- initial version
