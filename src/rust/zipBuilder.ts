/**
 * Standards-Compliant PKZIP Archive Builder with UTF-8-BOM (EF BB BF) Encoding
 *
 * Generates a valid binary `.zip` archive in-browser and provides UTF-8-BOM
 * (`0xEF, 0xBB, 0xBF` / `\uFEFF`) file encoding for all CLI installations,
 * updates, uninstallations, and plugin templates across Linux and Windows.
 */

export interface ZipFileEntry {
  path: string; // e.g. "clibm/Cargo.toml" or "clibm/install.sh"
  content: string;
}

export const UTF8_BOM_CHAR = '\uFEFF';
export const UTF8_BOM_BYTES = new Uint8Array([0xef, 0xbb, 0xbf]);

/**
 * Guarantees that a string starts with the UTF-8 Byte Order Mark (`\uFEFF` / `EF BB BF`).
 */
export function ensureUtf8Bom(content: string): string {
  if (content.startsWith(UTF8_BOM_CHAR)) {
    return content;
  }
  return UTF8_BOM_CHAR + content;
}

/**
 * Strips a leading UTF-8 BOM character for clean display in code viewers while
 * keeping the underlying file/download UTF-8-BOM encoded.
 */
export function stripUtf8BomForDisplay(content: string): string {
  return content.startsWith(UTF8_BOM_CHAR) ? content.slice(1) : content;
}

/**
 * Encodes text as UTF-8 with an explicit 3-byte BOM (`0xEF, 0xBB, 0xBF`).
 */
export function encodeUtf8WithBom(content: string): Uint8Array {
  const encoder = new TextEncoder();
  const cleanContent = content.startsWith(UTF8_BOM_CHAR) ? content.slice(1) : content;
  const bodyBytes = encoder.encode(cleanContent);
  const out = new Uint8Array(UTF8_BOM_BYTES.length + bodyBytes.length);
  out.set(UTF8_BOM_BYTES, 0);
  out.set(bodyBytes, UTF8_BOM_BYTES.length);
  return out;
}

/**
 * Creates a Blob prefixed with the UTF-8 Byte Order Mark (`0xEF, 0xBB, 0xBF`).
 */
export function createUtf8BomBlob(
  content: string,
  mimeType = 'text/plain;charset=utf-8'
): Blob {
  const bytes = encodeUtf8WithBom(content);
  return new Blob([bytes.buffer as ArrayBuffer], { type: mimeType });
}

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[n] = c >>> 0;
  }
  return table;
})();

function crc32(bytes: Uint8Array): number {
  let crc = 0xffffffff;
  for (let i = 0; i < bytes.length; i++) {
    crc = CRC_TABLE[(crc ^ bytes[i]) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

export function buildZipArchive(files: ZipFileEntry[]): Blob {
  const encoder = new TextEncoder();
  const localFileParts: Uint8Array[] = [];
  const centralDirParts: Uint8Array[] = [];
  let offset = 0;

  for (const file of files) {
    const nameBytes = encoder.encode(file.path);
    // Encode all packaged files with UTF-8-BOM (EF BB BF) for cross-platform compatibility
    const dataBytes = encodeUtf8WithBom(file.content);
    const crc = crc32(dataBytes);
    const size = dataBytes.length;

    // Local File Header (30 bytes + filename)
    const localHeader = new Uint8Array(30 + nameBytes.length);
    const lv = new DataView(localHeader.buffer);
    lv.setUint32(0, 0x04034b50, true); // Local file header signature
    lv.setUint16(4, 20, true); // Version needed to extract (2.0)
    lv.setUint16(6, 0x0800, true); // General purpose bit flag (UTF-8)
    lv.setUint16(8, 0, true); // Compression method (0 = STORE)
    lv.setUint16(10, 0x4800, true); // Last mod file time
    lv.setUint16(12, 0x5964, true); // Last mod file date
    lv.setUint32(14, crc, true); // CRC-32
    lv.setUint32(18, size, true); // Compressed size
    lv.setUint32(22, size, true); // Uncompressed size
    lv.setUint16(26, nameBytes.length, true); // File name length
    lv.setUint16(28, 0, true); // Extra field length
    localHeader.set(nameBytes, 30);

    localFileParts.push(localHeader, dataBytes);

    // Central Directory File Header (46 bytes + filename)
    const centralHeader = new Uint8Array(46 + nameBytes.length);
    const cv = new DataView(centralHeader.buffer);
    cv.setUint32(0, 0x02014b50, true); // Central directory signature
    cv.setUint16(4, 20, true); // Version made by
    cv.setUint16(6, 20, true); // Version needed to extract
    cv.setUint16(8, 0x0800, true); // General purpose bit flag (UTF-8)
    cv.setUint16(10, 0, true); // Compression method (0 = STORE)
    cv.setUint16(12, 0x4800, true); // File time
    cv.setUint16(14, 0x5964, true); // File date
    cv.setUint32(16, crc, true); // CRC-32
    cv.setUint32(20, size, true); // Compressed size
    cv.setUint32(24, size, true); // Uncompressed size
    cv.setUint16(28, nameBytes.length, true); // File name length
    cv.setUint16(30, 0, true); // Extra field length
    cv.setUint16(32, 0, true); // File comment length
    cv.setUint16(34, 0, true); // Disk number start
    cv.setUint16(36, 0, true); // Internal file attributes
    cv.setUint32(38, 0, true); // External file attributes
    cv.setUint32(42, offset, true); // Relative offset of local header
    centralHeader.set(nameBytes, 46);

    centralDirParts.push(centralHeader);
    offset += localHeader.length + dataBytes.length;
  }

  let centralDirSize = 0;
  for (const part of centralDirParts) {
    centralDirSize += part.length;
  }

  // End of Central Directory Record (22 bytes)
  const eocd = new Uint8Array(22);
  const ev = new DataView(eocd.buffer);
  ev.setUint32(0, 0x06054b50, true); // EOCD signature
  ev.setUint16(4, 0, true); // Number of this disk
  ev.setUint16(6, 0, true); // Disk where central directory starts
  ev.setUint16(8, files.length, true); // Total entries on this disk
  ev.setUint16(10, files.length, true); // Total entries in central directory
  ev.setUint32(12, centralDirSize, true); // Size of central directory
  ev.setUint32(16, offset, true); // Offset of start of central directory
  ev.setUint16(20, 0, true); // Comment length

  const allChunks = [...localFileParts, ...centralDirParts, eocd];
  const totalBytes = allChunks.reduce((sum, chunk) => sum + chunk.byteLength, 0);
  const combined = new Uint8Array(totalBytes);
  let writePos = 0;
  for (const chunk of allChunks) {
    combined.set(chunk, writePos);
    writePos += chunk.byteLength;
  }

  return new Blob([combined.buffer], {
    type: 'application/zip',
  });
}

export function triggerBlobDownload(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  setTimeout(() => URL.revokeObjectURL(url), 1500);
}

/**
 * Convenience helper to download any script or source file with UTF-8-BOM (EF BB BF) encoding.
 */
export function triggerUtf8BomFileDownload(
  content: string,
  filename: string,
  mimeType = 'text/plain;charset=utf-8'
): void {
  const blob = createUtf8BomBlob(content, mimeType);
  triggerBlobDownload(blob, filename);
}
