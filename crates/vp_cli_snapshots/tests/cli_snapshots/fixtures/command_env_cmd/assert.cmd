@echo off
setlocal

set "VP_ENV_USE_EVAL_ENABLE=caller-value"
set "VP_SHELL=caller-value"

call vp-use 20.18.0 --no-install
if errorlevel 1 (
  echo vp-use with an explicit version failed
  exit /b 1
)
if not "%VP_NODE_VERSION%"=="20.18.0" (
  echo explicit VP_NODE_VERSION mismatch: %VP_NODE_VERSION%
  exit /b 1
)
if exist "%VP_HOME%\.session-node-version" (
  echo explicit vp-use leaked a session file
  exit /b 1
)

call vp-use --unset
if errorlevel 1 (
  echo vp-use --unset failed
  exit /b 1
)
if defined VP_NODE_VERSION (
  echo vp-use --unset did not remove VP_NODE_VERSION
  exit /b 1
)
if exist "%VP_HOME%\.session-node-version" (
  echo vp-use --unset leaked a session file
  exit /b 1
)

call vp-use --no-install
if errorlevel 1 (
  echo vp-use without a version failed
  exit /b 1
)
if not "%VP_NODE_VERSION%"=="22.18.0" (
  echo file-based VP_NODE_VERSION mismatch: %VP_NODE_VERSION%
  exit /b 1
)
if exist "%VP_HOME%\.session-node-version" (
  echo file-based vp-use leaked a session file
  exit /b 1
)

call vp-use --invalid-option >nul 2>&1
if not errorlevel 1 (
  echo vp-use did not preserve a failing command status
  exit /b 1
)
if not "%VP_NODE_VERSION%"=="22.18.0" (
  echo failing vp-use changed VP_NODE_VERSION: %VP_NODE_VERSION%
  exit /b 1
)
if not "%VP_ENV_USE_EVAL_ENABLE%"=="caller-value" (
  echo vp-use changed the caller's VP_ENV_USE_EVAL_ENABLE
  exit /b 1
)
if not "%VP_SHELL%"=="caller-value" (
  echo vp-use changed the caller's VP_SHELL
  exit /b 1
)

echo Command Prompt environment use checks passed
