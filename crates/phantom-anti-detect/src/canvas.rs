use uuid::Uuid;

/// Generates a JS shim that hooks the Canvas API to inject deterministic 1-bit noise.
/// The noise is unique per session ID to frustrate cross-session tracking while
/// remaining consistent within a single session (avoiding easy detection of randomization).
pub fn generate_noise_shim(session_id: Uuid) -> String {
    let seed_str = session_id.to_string();
    format!(
        r#"
(function() {{
    const SEED = '{seed}';
    
    // Simple deterministic hash function for noise bits
    function getNoiseBit(x, y, channel) {{
        let str = SEED + x + ',' + y + ',' + channel;
        let hash = 0;
        for (let i = 0; i < str.length; i++) {{
            hash = ((hash << 5) - hash) + str.charCodeAt(i);
            hash |= 0;
        }}
        return hash & 1;
    }}

    const originalGetImageData = CanvasRenderingContext2D.prototype.getImageData;
    CanvasRenderingContext2D.prototype.getImageData = function(x, y, w, h) {{
        const imageData = originalGetImageData.apply(this, arguments);
        const data = imageData.data;
        for (let i = 0; i < data.length; i += 4) {{
            const px = (i / 4) % w;
            const py = Math.floor((i / 4) / w);
            // Inject 1-bit noise into the Red channel
            data[i] = data[i] ^ getNoiseBit(px + x, py + y, 0);
        }}
        return imageData;
    }};

    const originalToDataURL = HTMLCanvasElement.prototype.toDataURL;
    HTMLCanvasElement.prototype.toDataURL = function() {{
        // Force noise injection by performing a read-back and write-back if needed,
        // but typically getImageData hook is enough for most sophisticated scripts.
        // For toDataURL, we might need to subtly offset the drawing or add noise
        // to a temporary buffer.
        console.debug('Phantom: Canvas.toDataURL intercepted');
        return originalToDataURL.apply(this, arguments);
    }};

    console.debug('Phantom: Canvas noise shims initialized.');
}})();
"#,
        seed = seed_str
    )
}
