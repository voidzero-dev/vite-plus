@echo off
echo launcher=cmd
node assert.cjs %*
exit /b %ERRORLEVEL%
