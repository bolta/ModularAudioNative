# dx serve/build bundles gui_controller.exe under target/dx/gui_controller/... but does not
# copy the matching gui_controller.pdb there, so a debugger attached to the bundled process
# cannot resolve any symbols and breakpoints stay "pending". This script finds the pdb from
# cargo's actual build output (the "desktop-dev" profile dx uses) and keeps copying it into
# the bundle directory every time dx rebuilds.
param(
	[Parameter(Mandatory = $true)]
	[string]$RepoRoot
)

$srcPdb = Join-Path $RepoRoot "target\x86_64-pc-windows-msvc\desktop-dev\gui_controller.pdb"
$destDir = Join-Path $RepoRoot "target\dx\gui_controller\debug\windows\app"
$destPdb = Join-Path $destDir "gui_controller.pdb"

Write-Host "[copy-pdb] watching $srcPdb"

$lastWrite = $null
while ($true) {
	if ((Test-Path $srcPdb) -and (Test-Path $destDir)) {
		$current = (Get-Item $srcPdb).LastWriteTimeUtc
		# Re-copy whenever the source pdb changed, or whenever dx has recreated the bundle
		# directory (e.g. a fresh `dx serve` session) and dropped our previous copy, even if
		# the source pdb itself is unchanged because cargo's build was fully cached.
		if (($current -ne $lastWrite) -or -not (Test-Path $destPdb)) {
			try {
				Copy-Item $srcPdb $destPdb -Force -ErrorAction Stop
				$lastWrite = $current
				Write-Host "[copy-pdb] $(Get-Date -Format 'HH:mm:ss') updated gui_controller.pdb"
			} catch {
				# Linker may still be writing the file; retry on the next tick.
			}
		}
	}
	Start-Sleep -Seconds 1
}
