//! Browser shims injected BEFORE any page scripts.
//!
//! These JS snippets make the QuickJS environment look like a real Chrome
//! browser to anti-bot detection systems.

use crate::quickjs::bindings::navigator::Persona;

/// Generate the complete browser shim JavaScript string.
///
/// This JS is injected into the QuickJS context BEFORE any page scripts run.
/// It must make the environment indistinguishable from a real Chrome browser.
///
/// # Anti-Detection Measures
///
/// 1. `navigator.webdriver` → `undefined` (not false, not null)
/// 2. `window.chrome` object with runtime/app/csi stubs
/// 3. `navigator.plugins` with Chrome PDF plugins
/// 4. Persona-specific values (user agent, hardware concurrency, screen size)
/// 5. `performance.now()` timing jitter (±0.1ms noise)
/// 6. CDP artifact removal (`$cdc_` properties)
pub fn generate_shims(persona: &Persona) -> String {
    format!(
        r#"
// ============================================================
// Phantom Engine Browser Shims — injected before page scripts
// ============================================================

// Initialize browser globals that QuickJS does not provide
if (typeof globalThis.window === 'undefined') {{ globalThis.window = globalThis; }}
if (typeof globalThis.navigator === 'undefined') {{ globalThis.navigator = {{}}; }}
if (typeof globalThis.screen === 'undefined') {{ globalThis.screen = {{}}; }}
if (typeof globalThis.performance === 'undefined') {{ globalThis.performance = {{ now: function() {{ return Date.now(); }} }}; }}
var navigator = globalThis.navigator;
var window = globalThis.window;
var screen = globalThis.screen;

// 1. navigator.webdriver = undefined (CRITICAL)
// Must be undefined, not false, not null.
Object.defineProperty(navigator, 'webdriver', {{
    get: function() {{ return undefined; }},
    configurable: true
}});

// 2. window.chrome object
if (typeof window.chrome === 'undefined') {{
    window.chrome = {{
        runtime: {{ id: undefined }},
        app: {{ isInstalled: false }},
        csi: function() {{ return {{ startE: Date.now() }}; }},
    }};
}}

// 3. navigator.plugins
Object.defineProperty(navigator, 'plugins', {{
    get: function() {{
        return [
            {{ name: 'Chrome PDF Plugin', filename: 'internal-pdf-viewer', description: 'Portable Document Format' }},
            {{ name: 'Chrome PDF Viewer', filename: 'mhjfbmdgcfjbbpaeojofohoefgiehjai', description: '' }},
        ];
    }},
    configurable: true
}});

// 4. Persona-specific values
Object.defineProperty(navigator, 'hardwareConcurrency', {{
    get: function() {{ return {hardware_concurrency}; }},
    configurable: true
}});

Object.defineProperty(navigator, 'userAgent', {{
    get: function() {{ return '{user_agent}'; }},
    configurable: true
}});

Object.defineProperty(navigator, 'language', {{
    get: function() {{ return '{language}'; }},
    configurable: true
}});

Object.defineProperty(navigator, 'languages', {{
    get: function() {{ return ['{language}', '{language_base}']; }},
    configurable: true
}});

Object.defineProperty(navigator, 'platform', {{
    get: function() {{ return '{platform}'; }},
    configurable: true
}});

Object.defineProperty(navigator, 'onLine', {{
    get: function() {{ return true; }},
    configurable: true
}});

// Screen dimensions from persona
if (typeof screen !== 'undefined') {{
    Object.defineProperty(screen, 'width', {{
        get: function() {{ return {screen_width}; }},
        configurable: true
    }});
    Object.defineProperty(screen, 'height', {{
        get: function() {{ return {screen_height}; }},
        configurable: true
    }});
    Object.defineProperty(screen, 'availWidth', {{
        get: function() {{ return {screen_width}; }},
        configurable: true
    }});
    Object.defineProperty(screen, 'availHeight', {{
        get: function() {{ return {screen_height}; }},
        configurable: true
    }});
}}

// 5. Timing jitter — add ±0.1ms noise to performance.now()
(function() {{
    try {{
        if (typeof performance !== 'undefined' && performance.now) {{
            var _originalNow = performance.now.bind(performance);
            Object.defineProperty(performance, 'now', {{
                value: function() {{
                    return _originalNow() + (Math.random() * 0.2 - 0.1);
                }},
                writable: true,
                configurable: true
            }});
        }}
    }} catch(e) {{
        // performance.now override not critical — skip if not possible
    }}
}})();

// 6. Remove CDP artifacts
(function() {{
    var keys = Object.getOwnPropertyNames(window);
    for (var i = 0; i < keys.length; i++) {{
        if (keys[i].indexOf('$cdc_') === 0 || keys[i].indexOf('__driver') === 0) {{
            try {{ delete window[keys[i]]; }} catch(e) {{}}
        }}
    }}
}})();

// Polyfill console if not available
if (typeof console === 'undefined') {{
    var console = {{
        log: function() {{}},
        warn: function() {{}},
        error: function() {{}},
        info: function() {{}},
        debug: function() {{}}
    }};
}}
"#,
        hardware_concurrency = persona.hardware_concurrency,
        user_agent = persona.user_agent,
        language = persona.language,
        language_base = persona.language.split('-').next().unwrap_or("en"),
        platform = persona.platform,
        screen_width = persona.screen_width,
        screen_height = persona.screen_height,
    )
}
