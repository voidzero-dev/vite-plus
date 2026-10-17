@echo off
node "%~dp0fake-tool.js" %*
exit /b %ERRORLEVEL%
