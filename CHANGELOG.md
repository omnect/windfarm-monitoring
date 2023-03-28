# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] Q1 2023
 - metrics_provider:
   - reduced actix-web workers to 1
   - refactored default address handling
   - refactored drop behaviour

## [0.2.0] Q1 2023
 - replaced warp by actix-web
 - replaced BIND_ADDR and PORT by BIND_ADDR_AND_PORT

## [0.1.17] Q1 2023
 - fixed GHSA-4q83-7cq4-p6wg (explicit `cargo update`)

## [0.1.16] Q1 2023
 - updated dependencies in order to fix https://rustsec.org/advisories/RUSTSEC-2023-0018
 - refactored to latest iot-client-template-rs design
 - bumped to azure-iot-sdk 0.9.2

## [0.1.15] Q1 2023
 - updated tokio to 1.23 in order to fix cargo audit warning

## [0.1.14] Q4 2022
 - updated dependencies in order to fix https://github.com/tikv/rust-prometheus/issues/442
 - bumped to azure-iot-sdk 0.8.8

## [0.1.13] Q4 2022
 - renamed from ICS-DeviceManagement to omnect github orga
 - bumped to azure-iot-sdk 0.8.5

## [0.1.12] Q3 2022
 - fixed bug when async client does not terminate correctly
 - refactored thread handling of metrics provider
 - improved logging for AuthenticationStatus changes

## [0.1.11] Q3 2022
 - log message with severity error on panics

## [0.1.10] Q3 2022
 - fixed report azure-sdk-version in twin
 - switched from forked sd-notify to new official release 0.4.1
 - changed some debug messages to log level info

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
