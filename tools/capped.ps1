# Runs a command line (cmd.exe syntax) inside a Windows job object that caps the whole process
# tree at 50% of the CPU and 3.5 GB of committed memory. Output goes to -Log.
param(
    [Parameter(Mandatory = $true)][string]$Command,
    [Parameter(Mandatory = $true)][string]$Log,
    [string]$WorkDir = "C:\Users\joasg\GitHub\open-3d-viewer",
    [double]$MemoryGB = 3.5,
    [int]$CpuPercent = 50
)

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class JobCap {
    [StructLayout(LayoutKind.Sequential)] struct BASIC { public long PerProcessUserTimeLimit; public long PerJobUserTimeLimit; public uint LimitFlags; public UIntPtr MinimumWorkingSetSize; public UIntPtr MaximumWorkingSetSize; public uint ActiveProcessLimit; public UIntPtr Affinity; public uint PriorityClass; public uint SchedulingClass; }
    [StructLayout(LayoutKind.Sequential)] struct IO { public ulong a, b, c, d, e, f; }
    [StructLayout(LayoutKind.Sequential)] struct EXTENDED { public BASIC Basic; public IO Io; public UIntPtr ProcessMemoryLimit; public UIntPtr JobMemoryLimit; public UIntPtr PeakProcessMemoryUsed; public UIntPtr PeakJobMemoryUsed; }
    [StructLayout(LayoutKind.Sequential)] struct CPU { public uint ControlFlags; public uint CpuRate; }
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] static extern IntPtr CreateJobObject(IntPtr a, string name);
    [DllImport("kernel32.dll")] static extern bool SetInformationJobObject(IntPtr job, int cls, ref EXTENDED info, uint size);
    [DllImport("kernel32.dll")] static extern bool SetInformationJobObject(IntPtr job, int cls, ref CPU info, uint size);
    [DllImport("kernel32.dll")] static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
    [DllImport("kernel32.dll")] static extern bool QueryInformationJobObject(IntPtr job, int cls, out EXTENDED info, uint size, IntPtr ret);
    public static IntPtr Create(ulong memoryBytes, uint cpuRate) {
        IntPtr job = CreateJobObject(IntPtr.Zero, null);
        var ext = new EXTENDED();
        ext.Basic.LimitFlags = 0x200 | 0x2000; // JOB_MEMORY | KILL_ON_JOB_CLOSE
        ext.JobMemoryLimit = new UIntPtr(memoryBytes);
        if (!SetInformationJobObject(job, 9, ref ext, (uint)Marshal.SizeOf(ext))) throw new Exception("memory limit: " + Marshal.GetLastWin32Error());
        var cpu = new CPU { ControlFlags = 0x1 | 0x4, CpuRate = cpuRate }; // ENABLE | HARD_CAP
        if (!SetInformationJobObject(job, 15, ref cpu, (uint)Marshal.SizeOf(cpu))) throw new Exception("cpu limit: " + Marshal.GetLastWin32Error());
        return job;
    }
    public static bool Assign(IntPtr job, IntPtr process) { return AssignProcessToJobObject(job, process); }
    public static ulong Peak(IntPtr job) { EXTENDED e; QueryInformationJobObject(job, 9, out e, (uint)Marshal.SizeOf(typeof(EXTENDED)), IntPtr.Zero); return e.PeakJobMemoryUsed.ToUInt64(); }
}
"@

$job = [JobCap]::Create([uint64]($MemoryGB * 1GB), [uint32]($CpuPercent * 100))
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:CARGO_BUILD_JOBS = "2"
# a short wait first, so the process is in the job before it starts the build
$p = Start-Process -FilePath "cmd.exe" -ArgumentList "/c", "ping -n 2 127.0.0.1 >nul & $Command > `"$Log`" 2>&1" -WorkingDirectory $WorkDir -WindowStyle Hidden -PassThru
if (-not [JobCap]::Assign($job, $p.Handle)) { throw "could not put the build in the job object" }
$p.PriorityClass = 'BelowNormal'
$p.WaitForExit()
"exit $($p.ExitCode); peak memory of the build: {0:N2} GB" -f ([JobCap]::Peak($job) / 1GB)
