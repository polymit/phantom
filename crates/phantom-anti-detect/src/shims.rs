use crate::persona::Persona;

/// Generates a JavaScript string that patches common browser fingerprinting vectors.
/// This script should be executed in a privileged context before any site scripts.
pub fn generate_js_shims(persona: &Persona) -> String {
    format!(
        r#"
(function() {{
    // 1. Mask navigator.webdriver (must be undefined per D-22 spec)
    Object.defineProperty(navigator, 'webdriver', {{
        get: () => undefined
    }});

    // 2. Hardware constraints
    Object.defineProperty(navigator, 'hardwareConcurrency', {{
        get: () => {hardware_concurrency}
    }});

    Object.defineProperty(navigator, 'deviceMemory', {{
        get: () => {device_memory}
    }});

    // 3. Platform and User Agent
    Object.defineProperty(navigator, 'platform', {{
        get: () => '{platform}'
    }});

    Object.defineProperty(navigator, 'userAgent', {{
        get: () => '{user_agent}'
    }});

    // 4. Fake window.chrome structure (common detection vector)
    window.chrome = {{
        runtime: {{}},
        loadTimes: function() {{}},
        csi: function() {{}},
        app: {{}}
    }};

    // 5. Mask Plugins and MimeTypes
    Object.defineProperty(navigator, 'plugins', {{
        get: () => [
            {{ name: 'PDF Viewer', filename: 'internal-pdf-viewer', description: 'Portable Document Format' }},
            {{ name: 'Chrome PDF Viewer', filename: 'internal-pdf-viewer', description: 'Portable Document Format' }},
            {{ name: 'Chromium PDF Viewer', filename: 'internal-pdf-viewer', description: 'Portable Document Format' }},
            {{ name: 'Microsoft Edge PDF Viewer', filename: 'internal-pdf-viewer', description: 'Portable Document Format' }},
            {{ name: 'WebKit built-in PDF', filename: 'internal-pdf-viewer', description: 'Portable Document Format' }}
        ]
    }});
    
    Object.defineProperty(navigator, 'mimeTypes', {{
        get: () => [
            {{ type: 'application/pdf', suffixes: 'pdf', description: 'Portable Document Format', enabledPlugin: navigator.plugins[0] }}
        ]
    }});

    // 6. Fix for older detection scripts looking for CDC strings
    delete window.cdc_ado8s87asdf87s8asdf_Array;
    delete window.cdc_ado8s87asdf87s8asdf_Promise;
    delete window.cdc_ado8s87asdf87s8asdf_Symbol;

    // 7. Screen dimensions
    Object.defineProperty(window.screen, 'width', {{ get: () => {width} }});
    Object.defineProperty(window.screen, 'height', {{ get: () => {height} }});
    Object.defineProperty(window.screen, 'availWidth', {{ get: () => {width} }});
    Object.defineProperty(window.screen, 'availHeight', {{ get: () => {height} }});
    Object.defineProperty(window, 'innerWidth', {{ get: () => {width} }});
    Object.defineProperty(window, 'innerHeight', {{ get: () => {height} }});

    console.debug('Phantom anti-detect shims initialized for persona: {platform}');
}})();
"#,
        hardware_concurrency = persona.hardware_concurrency,
        device_memory = persona.device_memory,
        platform = persona.platform,
        user_agent = persona.user_agent,
        width = persona.screen_width,
        height = persona.screen_height
    )
}
