<#
  Kiln - Microsoft Office conversion helper.
  Invoked only by Kiln with validated arguments. Opens the input read-only in a hidden Office
  instance, saves it in the requested format, and always closes the document again.
  Exit code 0 = success, 1 = failure (message on stderr).
#>
param(
  [Parameter(Mandatory = $true)][ValidateSet('word', 'excel', 'powerpoint')][string]$App,
  [Parameter(Mandatory = $true)][string]$InFile,
  [Parameter(Mandatory = $true)][string]$OutFile,
  [Parameter(Mandatory = $true)][ValidateSet('pdf', 'docx', 'doc', 'odt', 'rtf', 'txt', 'html', 'xlsx', 'xls', 'ods', 'csv', 'pptx', 'ppt', 'odp', 'png', 'jpg')][string]$Format
)

$ErrorActionPreference = 'Stop'
# A password that is never correct: protected files then fail fast instead of showing a prompt.
$NoPassword = 'kiln-no-password-' + [guid]::NewGuid().ToString('N')

function Release($o) {
  if ($null -ne $o) { try { [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($o) } catch {} }
}

if (-not (Test-Path -LiteralPath $InFile -PathType Leaf)) { [Console]::Error.WriteLine("Input not found: $InFile"); exit 1 }
$InFile = (Resolve-Path -LiteralPath $InFile).ProviderPath

$office = $null; $doc = $null; $quit = $true

# Report the PID of the Office instance this script starts, so Kiln can stop exactly that
# process on cancel/timeout without touching Office windows the user has open.
function Start-Office([string]$progId, [string]$exe) {
  $before = @(Get-Process -Name $exe -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })
  $o = New-Object -ComObject $progId
  $new = @(Get-Process -Name $exe -ErrorAction SilentlyContinue | Where-Object { $before -notcontains $_.Id } | ForEach-Object { $_.Id })
  if ($new.Count -eq 1) { [Console]::Out.WriteLine("KILN_PID=$($new[0])"); [Console]::Out.Flush() }
  return $o
}

try {
  switch ($App) {
    'word' {
      $map = @{ pdf = 17; docx = 16; doc = 0; odt = 23; rtf = 6; txt = 2; html = 10 }
      if (-not $map.ContainsKey($Format)) { throw "Word cannot save as $Format" }
      # Word's PDF import blocks on a modal dialog in hidden automation; Kiln never sends PDFs here.
      if ($InFile -match '\.pdf$') { throw 'PDF input is not supported through Word automation' }
      $office = Start-Office 'Word.Application' 'WINWORD'
      $office.Visible = $false
      $office.DisplayAlerts = 0
      try { $office.Options.DoNotPromptForConvert = $true } catch {}
      # Open(FileName, ConfirmConversions, ReadOnly, AddToRecentFiles, PasswordDocument)
      $doc = $office.Documents.Open($InFile, $false, $true, $false, $NoPassword)
      if ($Format -eq 'txt') {
        # SaveAs2(FileName, FileFormat, LockComments, Password, AddToRecentFiles, WritePassword,
        #         ReadOnlyRecommended, EmbedTrueTypeFonts, SaveNativePictureFormat, SaveFormsData,
        #         SaveAsAOCELetter, Encoding=65001 UTF-8)
        $doc.SaveAs2($OutFile, 2, $false, '', $false, '', $false, $false, $false, $false, $false, 65001)
      } else {
        # Write HTML as UTF-8 (msoEncodingUTF8) instead of the legacy code page.
        if ($Format -eq 'html') { try { $doc.WebOptions.Encoding = 65001 } catch {} }
        $doc.SaveAs2($OutFile, $map[$Format])
      }
      $doc.Close(0)
    }
    'excel' {
      $map = @{ xlsx = 51; xls = 56; ods = 60; csv = 62 }
      $office = Start-Office 'Excel.Application' 'EXCEL'
      $office.Visible = $false
      $office.DisplayAlerts = $false
      $office.AskToUpdateLinks = $false
      # Open(FileName, UpdateLinks, ReadOnly, Format, Password)
      $doc = $office.Workbooks.Open($InFile, 0, $true, [Type]::Missing, $NoPassword)
      if ($Format -eq 'pdf') {
        $doc.ExportAsFixedFormat(0, $OutFile)
      } elseif ($map.ContainsKey($Format)) {
        try { $doc.SaveAs($OutFile, $map[$Format]) }
        catch {
          if ($Format -eq 'csv') { $doc.SaveAs($OutFile, 6) } else { throw }
        }
      } else { throw "Excel cannot save as $Format" }
      $doc.Close($false)
    }
    'powerpoint' {
      $map = @{ pptx = 24; ppt = 1; odp = 35; pdf = 32; png = 18; jpg = 17 }
      if (-not $map.ContainsKey($Format)) { throw "PowerPoint cannot save as $Format" }
      $office = New-Object -ComObject PowerPoint.Application
      # PowerPoint is a single shared instance: only quit it if nothing else was open.
      $quit = ($office.Presentations.Count -eq 0)
      # Open(FileName, ReadOnly=msoTrue, Untitled=msoFalse, WithWindow=msoFalse)
      $doc = $office.Presentations.Open($InFile, -1, 0, 0)
      $doc.SaveAs($OutFile, $map[$Format])
      $doc.Close()
    }
  }
  $doc = $null
  exit 0
}
catch {
  $msg = $_.Exception.Message
  if ($msg -match 'password') { $msg = 'The document is password protected' }
  [Console]::Error.WriteLine("Office conversion failed: $msg")
  exit 1
}
finally {
  if ($null -ne $doc) { try { if ($App -eq 'excel') { $doc.Close($false) } elseif ($App -eq 'word') { $doc.Close(0) } else { $doc.Close() } } catch {} }
  Release $doc
  if ($null -ne $office -and $quit) { try { $office.Quit() } catch {} }
  Release $office
  [GC]::Collect(); [GC]::WaitForPendingFinalizers()}
