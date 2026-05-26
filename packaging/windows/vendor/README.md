# Vendored Windows Utility

`nssm/nssm.exe` is the existing Non-Sucking Service Manager binary used to
install `KeyGenService.exe` as a Windows service.

```text
File version: 2.24-101-g897c7ad
SHA-256: EEE9C44C29C2BE011F1F1E43BB8C3FCA888CB81053022EC5A0060035DE16D848
```

Replacing this binary should be an explicit packaging change accompanied by
an updated checksum and service installation verification.
