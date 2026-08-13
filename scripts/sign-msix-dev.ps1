<#
.SYNOPSIS
  Sign the MSIX with a self-signed certificate so it can be installed locally for testing.

.DESCRIPTION
  Windows refuses to install an unsigned MSIX (0x800B010A). The Microsoft Store signs the package
  when you submit it, so no certificate is needed to publish - but that leaves no way to try the
  packaged build first. This creates a throwaway certificate whose subject matches the manifest's
  Publisher exactly (required, or Windows rejects the package identity) and signs a COPY of the
  .msix, leaving the original pristine for upload.

  Nothing here goes near the Store. The dev certificate is local, disposable, and gitignored.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\sign-msix-dev.ps1
#>

$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent

# The certificate subject MUST equal the manifest Publisher, or the signature won't match identity.
$manifest = Join-Path $root "src-tauri\msix\Package.appxmanifest"
$publisher = ([xml](Get-Content $manifest)).Package.Identity.Publisher
if (-not $publisher) { throw "Couldn't read Publisher from $manifest" }
Write-Host "Publisher: $publisher"

$msix = Get-ChildItem (Join-Path $root "Castline_*_x64.msix") |
        Where-Object { $_.Name -notlike "*dev-signed*" } |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $msix) { throw "No package found. Run: npm run pack:msix" }

$signed = $msix.FullName -replace "\.msix$", "-dev-signed.msix"
Copy-Item $msix.FullName $signed -Force

# Reuse the certificate across runs so the machine only has to trust it once.
$cert = Get-ChildItem Cert:\CurrentUser\My | Where-Object { $_.Subject -eq $publisher } |
        Sort-Object NotAfter -Descending | Select-Object -First 1
if (-not $cert) {
    Write-Host "Creating self-signed certificate..."
    $cert = New-SelfSignedCertificate -Type Custom -Subject $publisher `
        -KeyUsage DigitalSignature -FriendlyName "Castline dev signing (throwaway)" `
        -CertStoreLocation "Cert:\CurrentUser\My" `
        -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
} else {
    Write-Host "Reusing certificate $($cert.Thumbprint)"
}

$cer = Join-Path $root "devcert.cer"
Export-Certificate -Cert $cert -FilePath $cer -Force | Out-Null

$signtool = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe",
                          "C:\Program Files\Windows Kits\10\bin\*\x64\signtool.exe" -EA SilentlyContinue |
            Sort-Object FullName -Descending | Select-Object -First 1
if (-not $signtool) { throw "signtool.exe not found - install the Windows SDK." }

& $signtool.FullName sign /fd SHA256 /sha1 $cert.Thumbprint $signed
if ($LASTEXITCODE -ne 0) { throw "signtool failed" }

Write-Host ""
Write-Host "Signed: $signed" -ForegroundColor Green
Write-Host ""
Write-Host "To install, run these TWO commands in an ADMIN PowerShell:" -ForegroundColor Yellow
Write-Host "  Import-Certificate -FilePath `"$cer`" -CertStoreLocation Cert:\LocalMachine\TrustedPeople"
Write-Host "  Add-AppxPackage `"$signed`""
Write-Host ""
Write-Host "The first command only has to be run once. Afterwards, re-signing and"
Write-Host "re-running Add-AppxPackage is enough."
Write-Host ""
Write-Host "To remove it again:  Get-AppxPackage *Castline* | Remove-AppxPackage"
