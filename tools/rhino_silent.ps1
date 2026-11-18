# Runs a Rhino Python script SILENTLY: Rhino starts on a separate, invisible Windows desktop
# ("claude-silent"), so no window, splash or dialog of it can ever appear on the user's screen or
# take the focus. Waits for it to finish (or kills it, by its own process handle, after the
# timeout), then closes that desktop.
# usage: rhino_silent.ps1 -Script <path.py> [-Out <MODELS_OUT dir>] [-TimeoutSeconds 300]
param(
    [string]$Script = "",
    # start a Rhino that keeps running with the rhinomcp bridge on this port, instead of a script
    [int]$McpPort = 0,
    [string]$Out = "",
    [int]$TimeoutSeconds = 300
)
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class SilentDesktop {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    struct STARTUPINFO { public int cb; public string lpReserved; public string lpDesktop; public string lpTitle;
        public int dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars, dwFillAttribute, dwFlags;
        public short wShowWindow, cbReserved2; public IntPtr lpReserved2, hStdInput, hStdOutput, hStdError; }
    [StructLayout(LayoutKind.Sequential)]
    struct PROCESS_INFORMATION { public IntPtr hProcess, hThread; public int dwProcessId, dwThreadId; }
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern IntPtr CreateDesktop(string name, IntPtr device, IntPtr devmode, int flags, uint access, IntPtr sa);
    [DllImport("user32.dll")] static extern bool CloseDesktop(IntPtr desktop);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern bool CreateProcess(string app, string cmd, IntPtr pa, IntPtr ta, bool inherit, uint flags, IntPtr env, string dir, ref STARTUPINFO si, out PROCESS_INFORMATION pi);
    [DllImport("kernel32.dll")] static extern uint WaitForSingleObject(IntPtr h, uint ms);
    [DllImport("kernel32.dll")] static extern bool TerminateProcess(IntPtr h, uint code);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr h);
    // Runs `cmd` on the invisible desktop; returns "exited", "timeout (killed)" or an error.
    // Starts `cmd` on the invisible desktop and leaves it running; returns its process id.
    public static string Start(string app, string cmd, string dir) {
        const uint GENERIC_ALL = 0x10000000;
        IntPtr desk = CreateDesktop("claude-silent", IntPtr.Zero, IntPtr.Zero, 0, GENERIC_ALL, IntPtr.Zero);
        if (desk == IntPtr.Zero) return "CreateDesktop failed " + Marshal.GetLastWin32Error();
        var si = new STARTUPINFO(); si.cb = Marshal.SizeOf(si); si.lpDesktop = "claude-silent";
        si.dwFlags = 1; si.wShowWindow = 0;
        PROCESS_INFORMATION pi;
        if (!CreateProcess(app, cmd, IntPtr.Zero, IntPtr.Zero, false, 0, IntPtr.Zero, dir, ref si, out pi))
            return "CreateProcess failed " + Marshal.GetLastWin32Error();
        CloseHandle(pi.hThread); CloseHandle(pi.hProcess);
        // the desktop handle stays open until this PowerShell ends: by then Rhino uses the
        // desktop and keeps it alive
        return "started " + pi.dwProcessId;
    }
    public static string Run(string app, string cmd, string dir, uint timeoutMs) {
        const uint GENERIC_ALL = 0x10000000;
        IntPtr desk = CreateDesktop("claude-silent", IntPtr.Zero, IntPtr.Zero, 0, GENERIC_ALL, IntPtr.Zero);
        if (desk == IntPtr.Zero) return "CreateDesktop failed " + Marshal.GetLastWin32Error();
        try {
            var si = new STARTUPINFO(); si.cb = Marshal.SizeOf(si); si.lpDesktop = "claude-silent";
            si.dwFlags = 1; si.wShowWindow = 0; // STARTF_USESHOWWINDOW, SW_HIDE
            PROCESS_INFORMATION pi;
            if (!CreateProcess(app, cmd, IntPtr.Zero, IntPtr.Zero, false, 0, IntPtr.Zero, dir, ref si, out pi))
                return "CreateProcess failed " + Marshal.GetLastWin32Error();
            string result = "exited";
            if (WaitForSingleObject(pi.hProcess, timeoutMs) != 0) { TerminateProcess(pi.hProcess, 1); WaitForSingleObject(pi.hProcess, 10000); result = "timeout (killed)"; }
            CloseHandle(pi.hThread); CloseHandle(pi.hProcess);
            return result;
        } finally { CloseDesktop(desk); }
    }
}
"@
if ($Out) { $env:MODELS_OUT = $Out }
$rhino = "C:\Program Files\Rhino 9\System\Rhino.exe"
if ($McpPort -gt 0) {
    $env:RHINO_MCP_PORT = "$McpPort"
    $start = Join-Path $PSScriptRoot "rhino\mcp_start.py"
    $cmd = "`"$rhino`" /nosplash /runscript=`"_-RunPythonScript $start`""
    "rhino_silent: " + [SilentDesktop]::Start($rhino, $cmd, $env:TEMP) + " (rhinomcp bridge on port $McpPort)"
    # stay until the bridge listens (Rhino is then attached to the invisible desktop)
    $deadline = (Get-Date).AddSeconds(180)
    while ((Get-Date) -lt $deadline) {
        $c = New-Object System.Net.Sockets.TcpClient
        try { $c.Connect('127.0.0.1', $McpPort); $c.Close(); "rhino_silent: bridge listening on $McpPort"; return } catch { $c.Close() }
        [System.Threading.Thread]::Sleep(2000)
    }
    "rhino_silent: the bridge did not start within 3 minutes"
    return
}
$cmd = "`"$rhino`" /nosplash /runscript=`"_-RunPythonScript $Script _-Exit _No`""
$result = [SilentDesktop]::Run($rhino, $cmd, (Split-Path $Script), [uint32]($TimeoutSeconds * 1000))
"rhino_silent: $result ($Script)"
