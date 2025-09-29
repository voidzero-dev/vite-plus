use winapi::shared::minwindef::*;
use winapi::um::winnt::*;
use winapi::um::winnt::{INT};
use winapi::um::minwinbase::*;
use winapi::um::processthreadsapi::*;
use winapi::shared::guiddef::*;
use winapi::shared::windef::*;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _DETOUR_TRAMPOLINE {
    _unused: [u8; 0],
}
pub type PDETOUR_TRAMPOLINE = *mut _DETOUR_TRAMPOLINE;
#[doc = " Binary Typedefs."]
pub type PF_DETOUR_BINARY_BYWAY_CALLBACK = ::std::option::Option<
    unsafe extern "C" fn(pContext: PVOID, pszFile: LPCSTR, ppszOutFile: *mut LPCSTR) -> BOOL,
>;
pub type PF_DETOUR_BINARY_FILE_CALLBACK = ::std::option::Option<
    unsafe extern "C" fn(
        pContext: PVOID,
        pszOrigFile: LPCSTR,
        pszFile: LPCSTR,
        ppszOutFile: *mut LPCSTR,
    ) -> BOOL,
>;
pub type PF_DETOUR_BINARY_SYMBOL_CALLBACK = ::std::option::Option<
    unsafe extern "C" fn(
        pContext: PVOID,
        nOrigOrdinal: ULONG,
        nOrdinal: ULONG,
        pnOutOrdinal: *mut ULONG,
        pszOrigSymbol: LPCSTR,
        pszSymbol: LPCSTR,
        ppszOutSymbol: *mut LPCSTR,
    ) -> BOOL,
>;
pub type PF_DETOUR_BINARY_COMMIT_CALLBACK =
    ::std::option::Option<unsafe extern "C" fn(pContext: PVOID) -> BOOL>;
pub type PF_DETOUR_ENUMERATE_EXPORT_CALLBACK = ::std::option::Option<
    unsafe extern "C" fn(pContext: PVOID, nOrdinal: ULONG, pszName: LPCSTR, pCode: PVOID) -> BOOL,
>;
pub type PF_DETOUR_IMPORT_FILE_CALLBACK = ::std::option::Option<
    unsafe extern "C" fn(pContext: PVOID, hModule: HMODULE, pszFile: LPCSTR) -> BOOL,
>;
pub type PF_DETOUR_IMPORT_FUNC_CALLBACK = ::std::option::Option<
    unsafe extern "C" fn(pContext: PVOID, nOrdinal: DWORD, pszFunc: LPCSTR, pvFunc: PVOID) -> BOOL,
>;
pub type PF_DETOUR_IMPORT_FUNC_CALLBACK_EX = ::std::option::Option<
    unsafe extern "C" fn(
        pContext: PVOID,
        nOrdinal: DWORD,
        pszFunc: LPCSTR,
        ppvFunc: *mut PVOID,
    ) -> BOOL,
>;
pub type PDETOUR_BINARY = *mut ::std::os::raw::c_void;
unsafe extern "C" {
    #[doc = " Transaction APIs."]
    pub fn DetourTransactionBegin() -> LONG;
}
unsafe extern "C" {
    pub fn DetourTransactionAbort() -> LONG;
}
unsafe extern "C" {
    pub fn DetourTransactionCommit() -> LONG;
}
unsafe extern "C" {
    pub fn DetourTransactionCommitEx(pppFailedPointer: *mut *mut PVOID) -> LONG;
}
unsafe extern "C" {
    pub fn DetourUpdateThread(hThread: HANDLE) -> LONG;
}
unsafe extern "C" {
    pub fn DetourAttach(ppPointer: *mut PVOID, pDetour: PVOID) -> LONG;
}
unsafe extern "C" {
    pub fn DetourAttachEx(
        ppPointer: *mut PVOID,
        pDetour: PVOID,
        ppRealTrampoline: *mut PDETOUR_TRAMPOLINE,
        ppRealTarget: *mut PVOID,
        ppRealDetour: *mut PVOID,
    ) -> LONG;
}
unsafe extern "C" {
    pub fn DetourDetach(ppPointer: *mut PVOID, pDetour: PVOID) -> LONG;
}
unsafe extern "C" {
    pub fn DetourSetIgnoreTooSmall(fIgnore: BOOL) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourSetRetainRegions(fRetain: BOOL) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourSetSystemRegionLowerBound(pSystemRegionLowerBound: PVOID) -> PVOID;
}
unsafe extern "C" {
    pub fn DetourSetSystemRegionUpperBound(pSystemRegionUpperBound: PVOID) -> PVOID;
}
unsafe extern "C" {
    #[doc = " Code Functions."]
    pub fn DetourFindFunction(pszModule: LPCSTR, pszFunction: LPCSTR) -> PVOID;
}
unsafe extern "C" {
    pub fn DetourCodeFromPointer(pPointer: PVOID, ppGlobals: *mut PVOID) -> PVOID;
}
unsafe extern "C" {
    pub fn DetourCopyInstruction(
        pDst: PVOID,
        ppDstPool: *mut PVOID,
        pSrc: PVOID,
        ppTarget: *mut PVOID,
        plExtra: *mut LONG,
    ) -> PVOID;
}
unsafe extern "C" {
    pub fn DetourSetCodeModule(hModule: HMODULE, fLimitReferencesToModule: BOOL) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourAllocateRegionWithinJumpBounds(
        pbTarget: LPCVOID,
        pcbAllocatedSize: PDWORD,
    ) -> PVOID;
}
unsafe extern "C" {
    pub fn DetourIsFunctionImported(pbCode: PBYTE, pbAddress: PBYTE) -> BOOL;
}
unsafe extern "C" {
    #[doc = " Loaded Binary Functions."]
    pub fn DetourGetContainingModule(pvAddr: PVOID) -> HMODULE;
}
unsafe extern "C" {
    pub fn DetourEnumerateModules(hModuleLast: HMODULE) -> HMODULE;
}
unsafe extern "C" {
    pub fn DetourGetEntryPoint(hModule: HMODULE) -> PVOID;
}
unsafe extern "C" {
    pub fn DetourGetModuleSize(hModule: HMODULE) -> ULONG;
}
unsafe extern "C" {
    pub fn DetourEnumerateExports(
        hModule: HMODULE,
        pContext: PVOID,
        pfExport: PF_DETOUR_ENUMERATE_EXPORT_CALLBACK,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourEnumerateImports(
        hModule: HMODULE,
        pContext: PVOID,
        pfImportFile: PF_DETOUR_IMPORT_FILE_CALLBACK,
        pfImportFunc: PF_DETOUR_IMPORT_FUNC_CALLBACK,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourEnumerateImportsEx(
        hModule: HMODULE,
        pContext: PVOID,
        pfImportFile: PF_DETOUR_IMPORT_FILE_CALLBACK,
        pfImportFuncEx: PF_DETOUR_IMPORT_FUNC_CALLBACK_EX,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourFindPayload(hModule: HMODULE, rguid: *const GUID, pcbData: *mut DWORD) -> PVOID;
}
unsafe extern "C" {
    pub fn DetourFindPayloadEx(rguid: *const GUID, pcbData: *mut DWORD) -> PVOID;
}
unsafe extern "C" {
    pub fn DetourGetSizeOfPayloads(hModule: HMODULE) -> DWORD;
}
unsafe extern "C" {
    pub fn DetourFreePayload(pvData: PVOID) -> BOOL;
}
unsafe extern "C" {
    #[doc = " Persistent Binary Functions."]
    pub fn DetourBinaryOpen(hFile: HANDLE) -> PDETOUR_BINARY;
}
unsafe extern "C" {
    pub fn DetourBinaryEnumeratePayloads(
        pBinary: PDETOUR_BINARY,
        pGuid: *mut GUID,
        pcbData: *mut DWORD,
        pnIterator: *mut DWORD,
    ) -> PVOID;
}
unsafe extern "C" {
    pub fn DetourBinaryFindPayload(
        pBinary: PDETOUR_BINARY,
        rguid: *const GUID,
        pcbData: *mut DWORD,
    ) -> PVOID;
}
unsafe extern "C" {
    pub fn DetourBinarySetPayload(
        pBinary: PDETOUR_BINARY,
        rguid: *const GUID,
        pData: PVOID,
        cbData: DWORD,
    ) -> PVOID;
}
unsafe extern "C" {
    pub fn DetourBinaryDeletePayload(pBinary: PDETOUR_BINARY, rguid: *const GUID) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourBinaryPurgePayloads(pBinary: PDETOUR_BINARY) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourBinaryResetImports(pBinary: PDETOUR_BINARY) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourBinaryEditImports(
        pBinary: PDETOUR_BINARY,
        pContext: PVOID,
        pfByway: PF_DETOUR_BINARY_BYWAY_CALLBACK,
        pfFile: PF_DETOUR_BINARY_FILE_CALLBACK,
        pfSymbol: PF_DETOUR_BINARY_SYMBOL_CALLBACK,
        pfCommit: PF_DETOUR_BINARY_COMMIT_CALLBACK,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourBinaryWrite(pBinary: PDETOUR_BINARY, hFile: HANDLE) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourBinaryClose(pBinary: PDETOUR_BINARY) -> BOOL;
}
unsafe extern "C" {
    #[doc = " Create Process & Load Dll."]
    pub fn DetourFindRemotePayload(
        hProcess: HANDLE,
        rguid: *const GUID,
        pcbData: *mut DWORD,
    ) -> PVOID;
}
pub type PDETOUR_CREATE_PROCESS_ROUTINEA = ::std::option::Option<
    unsafe extern "C" fn(
        lpApplicationName: LPCSTR,
        lpCommandLine: LPSTR,
        lpProcessAttributes: LPSECURITY_ATTRIBUTES,
        lpThreadAttributes: LPSECURITY_ATTRIBUTES,
        bInheritHandles: BOOL,
        dwCreationFlags: DWORD,
        lpEnvironment: LPVOID,
        lpCurrentDirectory: LPCSTR,
        lpStartupInfo: LPSTARTUPINFOA,
        lpProcessInformation: LPPROCESS_INFORMATION,
    ) -> BOOL,
>;
pub type PDETOUR_CREATE_PROCESS_ROUTINEW = ::std::option::Option<
    unsafe extern "C" fn(
        lpApplicationName: LPCWSTR,
        lpCommandLine: LPWSTR,
        lpProcessAttributes: LPSECURITY_ATTRIBUTES,
        lpThreadAttributes: LPSECURITY_ATTRIBUTES,
        bInheritHandles: BOOL,
        dwCreationFlags: DWORD,
        lpEnvironment: LPVOID,
        lpCurrentDirectory: LPCWSTR,
        lpStartupInfo: LPSTARTUPINFOW,
        lpProcessInformation: LPPROCESS_INFORMATION,
    ) -> BOOL,
>;
unsafe extern "C" {
    pub fn DetourCreateProcessWithDllA(
        lpApplicationName: LPCSTR,
        lpCommandLine: LPSTR,
        lpProcessAttributes: LPSECURITY_ATTRIBUTES,
        lpThreadAttributes: LPSECURITY_ATTRIBUTES,
        bInheritHandles: BOOL,
        dwCreationFlags: DWORD,
        lpEnvironment: LPVOID,
        lpCurrentDirectory: LPCSTR,
        lpStartupInfo: LPSTARTUPINFOA,
        lpProcessInformation: LPPROCESS_INFORMATION,
        lpDllName: LPCSTR,
        pfCreateProcessA: PDETOUR_CREATE_PROCESS_ROUTINEA,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourCreateProcessWithDllW(
        lpApplicationName: LPCWSTR,
        lpCommandLine: LPWSTR,
        lpProcessAttributes: LPSECURITY_ATTRIBUTES,
        lpThreadAttributes: LPSECURITY_ATTRIBUTES,
        bInheritHandles: BOOL,
        dwCreationFlags: DWORD,
        lpEnvironment: LPVOID,
        lpCurrentDirectory: LPCWSTR,
        lpStartupInfo: LPSTARTUPINFOW,
        lpProcessInformation: LPPROCESS_INFORMATION,
        lpDllName: LPCSTR,
        pfCreateProcessW: PDETOUR_CREATE_PROCESS_ROUTINEW,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourCreateProcessWithDllExA(
        lpApplicationName: LPCSTR,
        lpCommandLine: LPSTR,
        lpProcessAttributes: LPSECURITY_ATTRIBUTES,
        lpThreadAttributes: LPSECURITY_ATTRIBUTES,
        bInheritHandles: BOOL,
        dwCreationFlags: DWORD,
        lpEnvironment: LPVOID,
        lpCurrentDirectory: LPCSTR,
        lpStartupInfo: LPSTARTUPINFOA,
        lpProcessInformation: LPPROCESS_INFORMATION,
        lpDllName: LPCSTR,
        pfCreateProcessA: PDETOUR_CREATE_PROCESS_ROUTINEA,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourCreateProcessWithDllExW(
        lpApplicationName: LPCWSTR,
        lpCommandLine: LPWSTR,
        lpProcessAttributes: LPSECURITY_ATTRIBUTES,
        lpThreadAttributes: LPSECURITY_ATTRIBUTES,
        bInheritHandles: BOOL,
        dwCreationFlags: DWORD,
        lpEnvironment: LPVOID,
        lpCurrentDirectory: LPCWSTR,
        lpStartupInfo: LPSTARTUPINFOW,
        lpProcessInformation: LPPROCESS_INFORMATION,
        lpDllName: LPCSTR,
        pfCreateProcessW: PDETOUR_CREATE_PROCESS_ROUTINEW,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourCreateProcessWithDllsA(
        lpApplicationName: LPCSTR,
        lpCommandLine: LPSTR,
        lpProcessAttributes: LPSECURITY_ATTRIBUTES,
        lpThreadAttributes: LPSECURITY_ATTRIBUTES,
        bInheritHandles: BOOL,
        dwCreationFlags: DWORD,
        lpEnvironment: LPVOID,
        lpCurrentDirectory: LPCSTR,
        lpStartupInfo: LPSTARTUPINFOA,
        lpProcessInformation: LPPROCESS_INFORMATION,
        nDlls: DWORD,
        rlpDlls: *mut LPCSTR,
        pfCreateProcessA: PDETOUR_CREATE_PROCESS_ROUTINEA,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourCreateProcessWithDllsW(
        lpApplicationName: LPCWSTR,
        lpCommandLine: LPWSTR,
        lpProcessAttributes: LPSECURITY_ATTRIBUTES,
        lpThreadAttributes: LPSECURITY_ATTRIBUTES,
        bInheritHandles: BOOL,
        dwCreationFlags: DWORD,
        lpEnvironment: LPVOID,
        lpCurrentDirectory: LPCWSTR,
        lpStartupInfo: LPSTARTUPINFOW,
        lpProcessInformation: LPPROCESS_INFORMATION,
        nDlls: DWORD,
        rlpDlls: *mut LPCSTR,
        pfCreateProcessW: PDETOUR_CREATE_PROCESS_ROUTINEW,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourProcessViaHelperA(
        dwTargetPid: DWORD,
        lpDllName: LPCSTR,
        pfCreateProcessA: PDETOUR_CREATE_PROCESS_ROUTINEA,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourProcessViaHelperW(
        dwTargetPid: DWORD,
        lpDllName: LPCSTR,
        pfCreateProcessW: PDETOUR_CREATE_PROCESS_ROUTINEW,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourProcessViaHelperDllsA(
        dwTargetPid: DWORD,
        nDlls: DWORD,
        rlpDlls: *mut LPCSTR,
        pfCreateProcessA: PDETOUR_CREATE_PROCESS_ROUTINEA,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourProcessViaHelperDllsW(
        dwTargetPid: DWORD,
        nDlls: DWORD,
        rlpDlls: *mut LPCSTR,
        pfCreateProcessW: PDETOUR_CREATE_PROCESS_ROUTINEW,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourUpdateProcessWithDll(hProcess: HANDLE, rlpDlls: *mut LPCSTR, nDlls: DWORD)
    -> BOOL;
}
unsafe extern "C" {
    pub fn DetourUpdateProcessWithDllEx(
        hProcess: HANDLE,
        hImage: HMODULE,
        bIs32Bit: BOOL,
        rlpDlls: *mut LPCSTR,
        nDlls: DWORD,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourCopyPayloadToProcess(
        hProcess: HANDLE,
        rguid: *const GUID,
        pvData: LPCVOID,
        cbData: DWORD,
    ) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourCopyPayloadToProcessEx(
        hProcess: HANDLE,
        rguid: *const GUID,
        pvData: LPCVOID,
        cbData: DWORD,
    ) -> PVOID;
}
unsafe extern "C" {
    pub fn DetourRestoreAfterWith() -> BOOL;
}
unsafe extern "C" {
    pub fn DetourRestoreAfterWithEx(pvData: PVOID, cbData: DWORD) -> BOOL;
}
unsafe extern "C" {
    pub fn DetourIsHelperProcess() -> BOOL;
}
unsafe extern "C" {
    pub fn DetourFinishHelperProcess(arg1: HWND, arg2: HINSTANCE, arg3: LPSTR, arg4: INT);
}
