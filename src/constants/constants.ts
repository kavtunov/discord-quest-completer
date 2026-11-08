import { InjectionKey } from "vue";

export const EXECUTABLE_OS = {
    WINDOWS: 'win32',
    DARWIN: 'darwin',
    LINUX: 'linux',
    ANDROID: 'android',
    IOS: 'ios',
} as const;

export const GameActionsKey = Symbol() as InjectionKey<string>;

function detectPlatformString(): string {
    const tauriPlatform = (window as any).__TAURI_INTERNALS__?.platform;
    if (typeof tauriPlatform === 'string' && tauriPlatform.length > 0) {
        return tauriPlatform.toLowerCase();
    }

    const nav = navigator as Navigator & { userAgentData?: { platform?: string } };
    const uaPlatform = nav.userAgentData?.platform;
    if (typeof uaPlatform === 'string' && uaPlatform.length > 0) {
        return uaPlatform.toLowerCase();
    }

    return (navigator.platform || navigator.userAgent || '').toLowerCase();
}

/** Discord detectable `os` field for the current host. */
export function getCurrentOS(): string {
    const platform = detectPlatformString();

    if (platform.includes('linux') || platform.includes('wayland') || platform.includes('x11')) {
        return EXECUTABLE_OS.LINUX;
    }
    if (platform.includes('win')) {
        return EXECUTABLE_OS.WINDOWS;
    }
    if (platform.includes('mac') || platform.includes('darwin')) {
        return EXECUTABLE_OS.DARWIN;
    }

    // Native Discord on this machine is Linux; default there for this fork.
    return EXECUTABLE_OS.LINUX;
}

export function isMacOS(): boolean {
    return getCurrentOS() === EXECUTABLE_OS.DARWIN;
}

export function isLinux(): boolean {
    return getCurrentOS() === EXECUTABLE_OS.LINUX;
}

/** Prefer native OS entries, then win32 (Discord still matches process names on Linux). */
export function fallbackExecutableOs(currentOs: string): string | null {
    if (currentOs === EXECUTABLE_OS.LINUX || currentOs === EXECUTABLE_OS.DARWIN) {
        return EXECUTABLE_OS.WINDOWS;
    }
    return null;
}
