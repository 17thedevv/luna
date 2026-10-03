import numpy as np
from PIL import Image, ImageFilter, ImageOps, ImageEnhance
import os

wordmark_path = r'C:\Users\84387\.gemini\antigravity-ide\brain\648ab9e4-b852-4837-96b6-c9272ddcd3c3\.user_uploaded\media_1790819316471.png'
symbol_path = r'C:\Users\84387\.gemini\antigravity-ide\brain\648ab9e4-b852-4837-96b6-c9272ddcd3c3\.user_uploaded\media_1790819413249.jpg'
out_dir = r'd:\fdlang\luna-web\src\assets\brand'
public_dir = r'd:\fdlang\luna-web\public'
os.makedirs(out_dir, exist_ok=True)
os.makedirs(public_dir, exist_ok=True)

# ---------------------------------------------------------
# 1. PROCESS SYMBOL (DARK & LIGHT)
# ---------------------------------------------------------
s_raw = Image.open(symbol_path).convert('RGB')
s_arr = np.array(s_raw, dtype=np.float32)

# Luminance calculation
s_lum = 0.299 * s_arr[:,:,0] + 0.587 * s_arr[:,:,1] + 0.114 * s_arr[:,:,2]

# Clean alpha: suppress background noise (values < 18)
# Soft knee between 18 and 50 to capture ethereal moonlight edge
bg_low = 18.0
bg_high = 52.0
alpha_curve = np.clip((s_lum - bg_low) / (bg_high - bg_low), 0.0, 1.0)
alpha_curve = alpha_curve * alpha_curve * (3.0 - 2.0 * alpha_curve) # smoothstep

# Also capture solid body (lum > 70 is fully opaque 1.0)
solid_curve = np.clip((s_lum - 50.0) / 40.0, 0.0, 1.0)
s_alpha = np.maximum(alpha_curve, solid_curve)
s_alpha_u8 = np.clip(s_alpha * 255.0, 0, 255).astype(np.uint8)

# Un-premultiply RGB against black background
alpha_norm = np.maximum(s_alpha[:, :, np.newaxis], 0.02)
s_rgb_clean = np.clip(s_arr / alpha_norm, 0, 255).astype(np.uint8)

# Construct Dark Symbol
s_dark = Image.fromarray(np.dstack([s_rgb_clean, s_alpha_u8]), 'RGBA')

# Crop to content with 16px padding
def get_bbox(img, min_a=15, pad=16):
    arr = np.array(img)[:,:,3]
    y_idx, x_idx = np.where(arr > min_a)
    return (max(0, x_idx.min() - pad), max(0, y_idx.min() - pad),
            min(img.width, x_idx.max() + pad), min(img.height, y_idx.max() + pad))

s_bbox = get_bbox(s_dark, min_a=20, pad=12)
s_dark = s_dark.crop(s_bbox)
s_dark.save(os.path.join(out_dir, 'luna-symbol-dark.png'))
print(f"Saved luna-symbol-dark.png {s_dark.size}")

# Construct Light Symbol:
# On light theme, moon and figure are deep obsidian-indigo (#0f172a / #1e1b4b)
# The laptop screen glows with vibrant lavender/violet (#8b5cf6 / #a78bfa)
s_arr_cropped = np.array(s_dark, dtype=np.float32)
rgb_c = s_arr_cropped[:, :, :3]
alpha_c = s_arr_cropped[:, :, 3]
lum_c = (0.299 * rgb_c[:,:,0] + 0.587 * rgb_c[:,:,1] + 0.114 * rgb_c[:,:,2]) / 255.0

# Detect laptop glow area: around figure's hands/lap with very high brightness
# Moon body: deep slate [15, 23, 42] with crater relief [45, 55, 80]
c_body = np.array([15.0, 23.0, 42.0])    # Slate 900
c_relief = np.array([45.0, 52.0, 75.0])  # Slate 700 with crater depth
c_glow = np.array([124.0, 58.0, 237.0])  # Violet 600 laptop glow

s_light_rgb = np.zeros_like(rgb_c)
for c in range(3):
    # Base dark silhouette with crater relief
    s_light_rgb[:, :, c] = c_body[c] + (1.0 - lum_c) * (c_relief[c] - c_body[c])

# For very bright spots (laptop screen), preserve violet/white luminosity
laptop_mask = np.clip((lum_c - 0.75) / 0.25, 0.0, 1.0)[:, :, np.newaxis]
screen_color = np.array([167.0, 139.0, 250.0]) # Soft lavender screen light
s_light_rgb = s_light_rgb * (1.0 - laptop_mask) + screen_color * laptop_mask

s_light = Image.fromarray(np.dstack([np.clip(s_light_rgb, 0, 255).astype(np.uint8), alpha_c.astype(np.uint8)]), 'RGBA')
s_light.save(os.path.join(out_dir, 'luna-symbol-light.png'))
print(f"Saved luna-symbol-light.png {s_light.size}")

# ---------------------------------------------------------
# 2. PROCESS WORDMARK (DARK & LIGHT)
# ---------------------------------------------------------
w_raw = Image.open(wordmark_path).convert('RGB')
w_arr = np.array(w_raw, dtype=np.float32)

w_lum = 0.299 * w_arr[:,:,0] + 0.587 * w_arr[:,:,1] + 0.114 * w_arr[:,:,2]

# Alpha for wordmark
w_t = np.clip((w_lum - 15.0) / 35.0, 0.0, 1.0)
w_alpha = w_t * w_t * (3.0 - 2.0 * w_t)
w_alpha = np.maximum(w_alpha, np.clip((w_lum - 40.0) / 50.0, 0.0, 1.0))
w_alpha_u8 = np.clip(w_alpha * 255.0, 0, 255).astype(np.uint8)

w_alpha_norm = np.maximum(w_alpha[:, :, np.newaxis], 0.02)
w_rgb_clean = np.clip(w_arr / w_alpha_norm, 0, 255).astype(np.uint8)

w_dark = Image.fromarray(np.dstack([w_rgb_clean, w_alpha_u8]), 'RGBA')
w_bbox = get_bbox(w_dark, min_a=20, pad=10)
w_dark = w_dark.crop(w_bbox)
w_dark.save(os.path.join(out_dir, 'luna-wordmark-dark.png'))
print(f"Saved luna-wordmark-dark.png {w_dark.size}")

# Construct Light Wordmark:
# Rich deep obsidian / slate (#0f172a) with subtle silver-slate crater relief (#334155 to #475569)
w_arr_c = np.array(w_dark, dtype=np.float32)
w_rgb_c = w_arr_c[:, :, :3]
w_a_c = w_arr_c[:, :, 3]
w_lum_c = (0.299 * w_rgb_c[:,:,0] + 0.587 * w_rgb_c[:,:,1] + 0.114 * w_rgb_c[:,:,2]) / 255.0

w_base = np.array([15.0, 23.0, 42.0])    # #0f172a Deep Slate
w_relief = np.array([51.0, 65.0, 85.0])  # #334155 Mid crater texture
w_hi = np.array([71.0, 85.0, 105.0])     # #475569 Bevel highlight

w_light_rgb = np.zeros_like(w_rgb_c)
for c in range(3):
    # Map texture while keeping high contrast against white
    w_light_rgb[:, :, c] = w_base[c] + w_lum_c * (w_hi[c] - w_base[c])

w_light = Image.fromarray(np.dstack([np.clip(w_light_rgb, 0, 255).astype(np.uint8), w_a_c.astype(np.uint8)]), 'RGBA')
w_light.save(os.path.join(out_dir, 'luna-wordmark-light.png'))
print(f"Saved luna-wordmark-light.png {w_light.size}")

# ---------------------------------------------------------
# 3. CREATE HORIZONTAL BRAND LOCKUPS (DARK & LIGHT)
# ---------------------------------------------------------
def create_lockup(symbol_img, wordmark_img, target_h=112):
    # Scale symbol to target_h
    s_aspect = symbol_img.width / symbol_img.height
    s_h = target_h
    s_w = int(s_h * s_aspect)
    s_scaled = symbol_img.resize((s_w, s_h), Image.Resampling.LANCZOS)

    # Scale wordmark to be optically balanced with symbol (wordmark height ~ 68% of symbol height)
    w_target_h = int(target_h * 0.64)
    w_aspect = wordmark_img.width / wordmark_img.height
    w_w = int(w_target_h * w_aspect)
    w_scaled = wordmark_img.resize((w_w, w_target_h), Image.Resampling.LANCZOS)

    # Gap between symbol and wordmark
    gap = int(target_h * 0.16)

    total_w = s_w + gap + w_w
    total_h = target_h

    canvas = Image.new('RGBA', (total_w, total_h), (0, 0, 0, 0))
    canvas.paste(s_scaled, (0, 0), s_scaled)

    # Center wordmark vertically with symbol
    w_y = (total_h - w_target_h) // 2 + int(target_h * 0.02) # optical baseline nudge
    canvas.paste(w_scaled, (s_w + gap, w_y), w_scaled)

    return canvas

lockup_dark = create_lockup(s_dark, w_dark, target_h=120)
lockup_dark.save(os.path.join(out_dir, 'luna-lockup-dark.png'))
print(f"Saved luna-lockup-dark.png {lockup_dark.size}")

lockup_light = create_lockup(s_light, w_light, target_h=120)
lockup_light.save(os.path.join(out_dir, 'luna-lockup-light.png'))
print(f"Saved luna-lockup-light.png {lockup_light.size}")

# Also copy lockups into src/assets/ for Starlight logo references
lockup_dark.save(r'd:\fdlang\luna-web\src\assets\logo-dark.png')
lockup_light.save(r'd:\fdlang\luna-web\src\assets\logo-light.png')

# ---------------------------------------------------------
# 4. CREATE FAVICONS & APP ICONS
# ---------------------------------------------------------
# High-contrast 64x64 and 32x32 favicon from symbol
s_sq = s_dark.resize((128, 128), Image.Resampling.LANCZOS)
s_sq.save(os.path.join(public_dir, 'favicon.png'))
s_32 = s_dark.resize((32, 32), Image.Resampling.LANCZOS)
s_32.save(os.path.join(public_dir, 'favicon-32x32.png'))
s_192 = s_dark.resize((192, 192), Image.Resampling.LANCZOS)
s_192.save(os.path.join(public_dir, 'apple-touch-icon.png'))
print("Favicons generated in public/")
