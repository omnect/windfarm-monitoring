# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
