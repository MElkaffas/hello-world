<#
  Kiln - prints the paths of the items selected in the File Explorer window whose handle is
  given. Used by the "convert Explorer selection" global shortcut. Read-only.
#>
param([Parameter(Mandatory = $true)][long]$Hwnd)
$ErrorActionPreference = 'SilentlyContinue'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$shell = New-Object -ComObject Shell.Application
foreach ($w in $shell.Windows()) {
  if ([long]$w.HWND -eq $Hwnd) {
    foreach ($item in $w.Document.SelectedItems()) { $item.Path }
    break
  }
}
