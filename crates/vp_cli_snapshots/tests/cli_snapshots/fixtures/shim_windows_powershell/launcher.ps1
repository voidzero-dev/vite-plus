Write-Output 'launcher=ps1'
& node (Join-Path $PWD 'assert.cjs') @args
exit $LASTEXITCODE
