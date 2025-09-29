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
    unsafe extern "system" fn(pContext: PVOID, pszFile: LPCSTR, ppszOutFile: *mut LPCSTR) -> BOOL,
>;
pub type PF_DETOUR_BINARY_FILE_CALLBACK = ::std::option::Option<
    unsafe extern "system" fn(
        pContext: PVOID,
        pszOrigFile: LPCSTR,
        pszFile: LPCSTR,
        ppszOutFile: *mut LPCSTR,
    ) -> BOOL,
>;
pub type PF_DETOUR_BINARY_SYMBOL_CALLBACK = ::std::option::Option<
    unsafe extern "system" fn(
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
    ::std::option::Option<unsafe extern "system" fn(pContext: PVOID) -> BOOL>;
pub type PF_DETOUR_ENUMERATE_EXPORT_CALLBACK = ::std::option::Option<
    unsafe extern "system" fn(
        pContext: PVOID,
        nOrdinal: ULONG,
        pszName: LPCSTR,
        pCode: PVOID,
    ) -> BOOL,
>;
pub type PF_DETOUR_IMPORT_FILE_CALLBACK = ::std::option::Option<
    unsafe extern "system" fn(pContext: PVOID, hModule: HMODULE, pszFile: LPCSTR) -> BOOL,
>;
pub type PF_DETOUR_IMPORT_FUNC_CALLBACK = ::std::option::Option<
    unsafe extern "system" fn(
        pContext: PVOID,
        nOrdinal: DWORD,
        pszFunc: LPCSTR,
        pvFunc: PVOID,
    ) -> BOOL,
>;
pub type PF_DETOUR_IMPORT_FUNC_CALLBACK_EX = ::std::option::Option<
    unsafe extern "system" fn(
        pContext: PVOID,
        nOrdinal: DWORD,
        pszFunc: LPCSTR,
        ppvFunc: *mut PVOID,
    ) -> BOOL,
>;
pub type PDETOUR_BINARY = *mut ::std::os::raw::c_void;
unsafe extern "system" {
    #[doc = " Transaction APIs."]
    #[link_name = "\u{1}_DetourTransactionBegin@0"]
    pub fn DetourTransactionBegin() -> LONG;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourTransactionAbort@0"]
    pub fn DetourTransactionAbort() -> LONG;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourTransactionCommit@0"]
    pub fn DetourTransactionCommit() -> LONG;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourTransactionCommitEx@4"]
    pub fn DetourTransactionCommitEx(pppFailedPointer: *mut *mut PVOID) -> LONG;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourUpdateThread@4"]
    pub fn DetourUpdateThread(hThread: HANDLE) -> LONG;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourAttach@8"]
    pub fn DetourAttach(ppPointer: *mut PVOID, pDetour: PVOID) -> LONG;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourAttachEx@20"]
    pub fn DetourAttachEx(
        ppPointer: *mut PVOID,
        pDetour: PVOID,
        ppRealTrampoline: *mut PDETOUR_TRAMPOLINE,
        ppRealTarget: *mut PVOID,
        ppRealDetour: *mut PVOID,
    ) -> LONG;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourDetach@8"]
    pub fn DetourDetach(ppPointer: *mut PVOID, pDetour: PVOID) -> LONG;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourSetIgnoreTooSmall@4"]
    pub fn DetourSetIgnoreTooSmall(fIgnore: BOOL) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourSetRetainRegions@4"]
    pub fn DetourSetRetainRegions(fRetain: BOOL) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourSetSystemRegionLowerBound@4"]
    pub fn DetourSetSystemRegionLowerBound(pSystemRegionLowerBound: PVOID) -> PVOID;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourSetSystemRegionUpperBound@4"]
    pub fn DetourSetSystemRegionUpperBound(pSystemRegionUpperBound: PVOID) -> PVOID;
}
unsafe extern "system" {
    #[doc = " Code Functions."]
    #[link_name = "\u{1}_DetourFindFunction@8"]
    pub fn DetourFindFunction(pszModule: LPCSTR, pszFunction: LPCSTR) -> PVOID;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourCodeFromPointer@8"]
    pub fn DetourCodeFromPointer(pPointer: PVOID, ppGlobals: *mut PVOID) -> PVOID;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourCopyInstruction@20"]
    pub fn DetourCopyInstruction(
        pDst: PVOID,
        ppDstPool: *mut PVOID,
        pSrc: PVOID,
        ppTarget: *mut PVOID,
        plExtra: *mut LONG,
    ) -> PVOID;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourSetCodeModule@8"]
    pub fn DetourSetCodeModule(hModule: HMODULE, fLimitReferencesToModule: BOOL) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourAllocateRegionWithinJumpBounds@8"]
    pub fn DetourAllocateRegionWithinJumpBounds(
        pbTarget: LPCVOID,
        pcbAllocatedSize: PDWORD,
    ) -> PVOID;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourIsFunctionImported@8"]
    pub fn DetourIsFunctionImported(pbCode: PBYTE, pbAddress: PBYTE) -> BOOL;
}
unsafe extern "system" {
    #[doc = " Loaded Binary Functions."]
    #[link_name = "\u{1}_DetourGetContainingModule@4"]
    pub fn DetourGetContainingModule(pvAddr: PVOID) -> HMODULE;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourEnumerateModules@4"]
    pub fn DetourEnumerateModules(hModuleLast: HMODULE) -> HMODULE;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourGetEntryPoint@4"]
    pub fn DetourGetEntryPoint(hModule: HMODULE) -> PVOID;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourGetModuleSize@4"]
    pub fn DetourGetModuleSize(hModule: HMODULE) -> ULONG;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourEnumerateExports@12"]
    pub fn DetourEnumerateExports(
        hModule: HMODULE,
        pContext: PVOID,
        pfExport: PF_DETOUR_ENUMERATE_EXPORT_CALLBACK,
    ) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourEnumerateImports@16"]
    pub fn DetourEnumerateImports(
        hModule: HMODULE,
        pContext: PVOID,
        pfImportFile: PF_DETOUR_IMPORT_FILE_CALLBACK,
        pfImportFunc: PF_DETOUR_IMPORT_FUNC_CALLBACK,
    ) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourEnumerateImportsEx@16"]
    pub fn DetourEnumerateImportsEx(
        hModule: HMODULE,
        pContext: PVOID,
        pfImportFile: PF_DETOUR_IMPORT_FILE_CALLBACK,
        pfImportFuncEx: PF_DETOUR_IMPORT_FUNC_CALLBACK_EX,
    ) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourFindPayload@12"]
    pub fn DetourFindPayload(hModule: HMODULE, rguid: *const GUID, pcbData: *mut DWORD) -> PVOID;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourFindPayloadEx@8"]
    pub fn DetourFindPayloadEx(rguid: *const GUID, pcbData: *mut DWORD) -> PVOID;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourGetSizeOfPayloads@4"]
    pub fn DetourGetSizeOfPayloads(hModule: HMODULE) -> DWORD;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourFreePayload@4"]
    pub fn DetourFreePayload(pvData: PVOID) -> BOOL;
}
unsafe extern "system" {
    #[doc = " Persistent Binary Functions."]
    #[link_name = "\u{1}_DetourBinaryOpen@4"]
    pub fn DetourBinaryOpen(hFile: HANDLE) -> PDETOUR_BINARY;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourBinaryEnumeratePayloads@16"]
    pub fn DetourBinaryEnumeratePayloads(
        pBinary: PDETOUR_BINARY,
        pGuid: *mut GUID,
        pcbData: *mut DWORD,
        pnIterator: *mut DWORD,
    ) -> PVOID;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourBinaryFindPayload@12"]
    pub fn DetourBinaryFindPayload(
        pBinary: PDETOUR_BINARY,
        rguid: *const GUID,
        pcbData: *mut DWORD,
    ) -> PVOID;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourBinarySetPayload@16"]
    pub fn DetourBinarySetPayload(
        pBinary: PDETOUR_BINARY,
        rguid: *const GUID,
        pData: PVOID,
        cbData: DWORD,
    ) -> PVOID;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourBinaryDeletePayload@8"]
    pub fn DetourBinaryDeletePayload(pBinary: PDETOUR_BINARY, rguid: *const GUID) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourBinaryPurgePayloads@4"]
    pub fn DetourBinaryPurgePayloads(pBinary: PDETOUR_BINARY) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourBinaryResetImports@4"]
    pub fn DetourBinaryResetImports(pBinary: PDETOUR_BINARY) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourBinaryEditImports@24"]
    pub fn DetourBinaryEditImports(
        pBinary: PDETOUR_BINARY,
        pContext: PVOID,
        pfByway: PF_DETOUR_BINARY_BYWAY_CALLBACK,
        pfFile: PF_DETOUR_BINARY_FILE_CALLBACK,
        pfSymbol: PF_DETOUR_BINARY_SYMBOL_CALLBACK,
        pfCommit: PF_DETOUR_BINARY_COMMIT_CALLBACK,
    ) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourBinaryWrite@8"]
    pub fn DetourBinaryWrite(pBinary: PDETOUR_BINARY, hFile: HANDLE) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourBinaryClose@4"]
    pub fn DetourBinaryClose(pBinary: PDETOUR_BINARY) -> BOOL;
}
unsafe extern "system" {
    #[doc = " Create Process & Load Dll."]
    #[link_name = "\u{1}_DetourFindRemotePayload@12"]
    pub fn DetourFindRemotePayload(
        hProcess: HANDLE,
        rguid: *const GUID,
        pcbData: *mut DWORD,
    ) -> PVOID;
}
pub type PDETOUR_CREATE_PROCESS_ROUTINEA = ::std::option::Option<
    unsafe extern "system" fn(
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
    unsafe extern "system" fn(
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
unsafe extern "system" {
    #[link_name = "\u{1}_DetourCreateProcessWithDllA@48"]
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
unsafe extern "system" {
    #[link_name = "\u{1}_DetourCreateProcessWithDllW@48"]
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
unsafe extern "system" {
    #[link_name = "\u{1}_DetourCreateProcessWithDllExA@48"]
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
unsafe extern "system" {
    #[link_name = "\u{1}_DetourCreateProcessWithDllExW@48"]
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
unsafe extern "system" {
    #[link_name = "\u{1}_DetourCreateProcessWithDllsA@52"]
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
unsafe extern "system" {
    #[link_name = "\u{1}_DetourCreateProcessWithDllsW@52"]
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
unsafe extern "system" {
    #[link_name = "\u{1}_DetourProcessViaHelperA@12"]
    pub fn DetourProcessViaHelperA(
        dwTargetPid: DWORD,
        lpDllName: LPCSTR,
        pfCreateProcessA: PDETOUR_CREATE_PROCESS_ROUTINEA,
    ) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourProcessViaHelperW@12"]
    pub fn DetourProcessViaHelperW(
        dwTargetPid: DWORD,
        lpDllName: LPCSTR,
        pfCreateProcessW: PDETOUR_CREATE_PROCESS_ROUTINEW,
    ) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourProcessViaHelperDllsA@16"]
    pub fn DetourProcessViaHelperDllsA(
        dwTargetPid: DWORD,
        nDlls: DWORD,
        rlpDlls: *mut LPCSTR,
        pfCreateProcessA: PDETOUR_CREATE_PROCESS_ROUTINEA,
    ) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourProcessViaHelperDllsW@16"]
    pub fn DetourProcessViaHelperDllsW(
        dwTargetPid: DWORD,
        nDlls: DWORD,
        rlpDlls: *mut LPCSTR,
        pfCreateProcessW: PDETOUR_CREATE_PROCESS_ROUTINEW,
    ) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourUpdateProcessWithDll@12"]
    pub fn DetourUpdateProcessWithDll(hProcess: HANDLE, rlpDlls: *mut LPCSTR, nDlls: DWORD)
    -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourUpdateProcessWithDllEx@20"]
    pub fn DetourUpdateProcessWithDllEx(
        hProcess: HANDLE,
        hImage: HMODULE,
        bIs32Bit: BOOL,
        rlpDlls: *mut LPCSTR,
        nDlls: DWORD,
    ) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourCopyPayloadToProcess@16"]
    pub fn DetourCopyPayloadToProcess(
        hProcess: HANDLE,
        rguid: *const GUID,
        pvData: LPCVOID,
        cbData: DWORD,
    ) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourCopyPayloadToProcessEx@16"]
    pub fn DetourCopyPayloadToProcessEx(
        hProcess: HANDLE,
        rguid: *const GUID,
        pvData: LPCVOID,
        cbData: DWORD,
    ) -> PVOID;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourRestoreAfterWith@0"]
    pub fn DetourRestoreAfterWith() -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourRestoreAfterWithEx@8"]
    pub fn DetourRestoreAfterWithEx(pvData: PVOID, cbData: DWORD) -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourIsHelperProcess@0"]
    pub fn DetourIsHelperProcess() -> BOOL;
}
unsafe extern "system" {
    #[link_name = "\u{1}_DetourFinishHelperProcess@16"]
    pub fn DetourFinishHelperProcess(arg1: HWND, arg2: HINSTANCE, arg3: LPSTR, arg4: INT);
}
