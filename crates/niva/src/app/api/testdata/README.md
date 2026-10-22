# HTTP TLS test identity

`http-untrusted-server.pfx` is a test-only self-signed TLS server identity for
the HTTP client tests. The private key is intentionally checked in so the test
can run without an `openssl` executable; it must never be installed as a trust
anchor, used outside tests, or used for production traffic.

The certificate subject is `CN=localhost`; its subject alternative names are
`DNS:localhost` and `IP:127.0.0.1`. It was generated on 2026-10-21 and is valid
through 2031-10-21. Regenerate the fixture before expiry and update this note.

To regenerate it on Windows, use PowerShell's built-in PKI cmdlets. Export the
PFX, then remove the temporary certificate from the current user's personal
store; do not print or commit the key separately:

```powershell
$password = ConvertTo-SecureString 'niva-http-test-fixture' -AsPlainText -Force
$cert = New-SelfSignedCertificate `
  -Subject 'CN=localhost' `
  -TextExtension @('2.5.29.17={text}DNS=localhost&IPAddress=127.0.0.1') `
  -CertStoreLocation 'Cert:\CurrentUser\My' `
  -KeyAlgorithm RSA `
  -KeyLength 2048 `
  -KeyExportPolicy Exportable `
  -KeyUsage DigitalSignature,KeyEncipherment `
  -Type SSLServerAuthentication `
  -NotAfter (Get-Date).AddYears(5)
try {
  Export-PfxCertificate -Cert $cert -FilePath .\http-untrusted-server.pfx -Password $password
} finally {
  Remove-Item -LiteralPath "Cert:\CurrentUser\My\$($cert.Thumbprint)"
}
```
