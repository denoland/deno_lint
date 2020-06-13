# Copyright 2018-2020 the Deno authors. All rights reserved. MIT license.
import collections
import os
import re
import sys
import subprocess

if os.environ.get("NO_COLOR", None):
    RESET = FG_READ = FG_GREEN = ""
else:
    RESET = "\x1b[0m"
    FG_RED = "\x1b[31m"
    FG_GREEN = "\x1b[32m"

root_path = os.path.dirname(os.path.dirname(os.path.realpath(__file__)))


def make_env(merge_env=None, env=None):
    if env is None:
        env = os.environ
    env = env.copy()
    if merge_env is None:
        merge_env = {}
    for key in merge_env.keys():
        env[key] = merge_env[key]
    return env


def run(args, quiet=False, cwd=None, env=None, merge_env=None, shell=None):
    args[0] = os.path.normpath(args[0])
    env = make_env(env=env, merge_env=merge_env)
    if shell is None:
        # Use the default value for 'shell' parameter.
        #   - Posix: do not use shell.
        #   - Windows: use shell; this makes .bat/.cmd files work.
        shell = os.name == "nt"
    if not quiet:
        print " ".join([shell_quote(arg) for arg in args])
    rc = subprocess.call(args, cwd=cwd, env=env, shell=shell)
    if rc != 0:
        sys.exit(rc)


def shell_quote_win(arg):
    if re.search(r'[\x00-\x20"^%~!@&?*<>|()=]', arg):
        # Double all " quote characters.
        arg = arg.replace('"', '""')
        # Wrap the entire string in " quotes.
        arg = '"' + arg + '"'
        # Double any N backslashes that are immediately followed by a " quote.
        arg = re.sub(r'(\\+)(?=")', r'\1\1', arg)
    return arg


def shell_quote(arg):
    if os.name == "nt":
        return shell_quote_win(arg)
    else:
        # Python 2 has posix shell quoting built in, albeit in a weird place.
        from pipes import quote
        return quote(arg)


# Attempts to enable ANSI escape code support.
# Returns True if successful, False if not supported.
def enable_ansi_colors():
    if os.name != 'nt':
        return True  # On non-windows platforms this just works.
    return enable_ansi_colors_win10()


# The windows 10 implementation of enable_ansi_colors.
def enable_ansi_colors_win10():
    import ctypes

    # Function factory for errcheck callbacks that raise WinError on failure.
    def raise_if(error_result):
        def check(result, _func, args):
            if result == error_result:
                raise ctypes.WinError(ctypes.get_last_error())
            return args

        return check

    # Windows API types.
    from ctypes.wintypes import BOOL, DWORD, HANDLE, LPCWSTR, LPVOID
    LPDWORD = ctypes.POINTER(DWORD)

    # Generic constants.
    NULL = ctypes.c_void_p(0).value
    INVALID_HANDLE_VALUE = ctypes.c_void_p(-1).value
    ERROR_INVALID_PARAMETER = 87

    # CreateFile flags.
    # yapf: disable
    GENERIC_READ = 0x80000000
    GENERIC_WRITE = 0x40000000
    FILE_SHARE_READ = 0x01
    FILE_SHARE_WRITE = 0x02
    OPEN_EXISTING = 3
    # yapf: enable

    # Get/SetConsoleMode flags.
    ENABLE_VIRTUAL_TERMINAL_PROCESSING = 0x04

    kernel32 = ctypes.WinDLL('kernel32', use_last_error=True)

    # HANDLE CreateFileW(...)
    CreateFileW = kernel32.CreateFileW
    CreateFileW.restype = HANDLE
    CreateFileW.errcheck = raise_if(INVALID_HANDLE_VALUE)
    # yapf: disable
    CreateFileW.argtypes = (LPCWSTR,  # lpFileName
                            DWORD,    # dwDesiredAccess
                            DWORD,    # dwShareMode
                            LPVOID,   # lpSecurityAttributes
                            DWORD,    # dwCreationDisposition
                            DWORD,    # dwFlagsAndAttributes
                            HANDLE)   # hTemplateFile
    # yapf: enable

    # BOOL CloseHandle(HANDLE hObject)
    CloseHandle = kernel32.CloseHandle
    CloseHandle.restype = BOOL
    CloseHandle.errcheck = raise_if(False)
    CloseHandle.argtypes = (HANDLE, )

    # BOOL GetConsoleMode(HANDLE hConsoleHandle, LPDWORD lpMode)
    GetConsoleMode = kernel32.GetConsoleMode
    GetConsoleMode.restype = BOOL
    GetConsoleMode.errcheck = raise_if(False)
    GetConsoleMode.argtypes = (HANDLE, LPDWORD)

    # BOOL SetConsoleMode(HANDLE hConsoleHandle, DWORD dwMode)
    SetConsoleMode = kernel32.SetConsoleMode
    SetConsoleMode.restype = BOOL
    SetConsoleMode.errcheck = raise_if(False)
    SetConsoleMode.argtypes = (HANDLE, DWORD)

    # Open the console output device.
    conout = CreateFileW("CONOUT$", GENERIC_READ | GENERIC_WRITE,
                         FILE_SHARE_READ | FILE_SHARE_WRITE, NULL,
                         OPEN_EXISTING, 0, 0)

    # Get the current mode.
    mode = DWORD()
    GetConsoleMode(conout, ctypes.byref(mode))

    # Try to set the flag that controls ANSI escape code support.
    try:
        SetConsoleMode(conout, mode.value | ENABLE_VIRTUAL_TERMINAL_PROCESSING)
    except WindowsError as e:  # pylint:disable=undefined-variable
        if e.winerror == ERROR_INVALID_PARAMETER:
            return False  # Not supported, likely an older version of Windows.
        raise
    finally:
        CloseHandle(conout)

    return True
