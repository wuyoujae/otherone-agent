$ErrorActionPreference = "Stop"

$packages = @(
    "otherone-ai",
    "otherone-storage",
    "otherone-memory",
    "otherone-tools",
    "otherone-skills",
    "otherone-mcp",
    "otherone-context",
    "otherone-agent",
    "otherone"
)

foreach ($package in $packages) {
    Write-Host "Publishing $package..."
    cargo publish -p $package --allow-dirty
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to publish $package"
    }
    Write-Host "Published $package"
}
