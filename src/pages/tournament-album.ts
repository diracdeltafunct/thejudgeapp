import { invoke } from "@tauri-apps/api/core";

function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve) => {
    const reader = new FileReader();
    reader.onload = () => resolve((reader.result as string).split(",")[1]);
    reader.readAsDataURL(blob);
  });
}

const DB_NAME = "judge-photos";
const DB_VERSION = 1;
const STORE = "photos";

interface Photo {
  id: string;
  tournamentId: string;
  blob: Blob;
  takenAt: string;
}

function openDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      const store = req.result.createObjectStore(STORE, { keyPath: "id" });
      store.createIndex("tournamentId", "tournamentId");
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function getPhotos(tournamentId: string): Promise<Photo[]> {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE, "readonly");
    const idx = tx.objectStore(STORE).index("tournamentId");
    const req = idx.getAll(tournamentId);
    req.onsuccess = () => resolve(req.result as Photo[]);
    req.onerror = () => reject(req.error);
  });
}

async function savePhoto(photo: Photo): Promise<void> {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE, "readwrite");
    tx.objectStore(STORE).put(photo);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

async function deletePhoto(id: string): Promise<void> {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE, "readwrite");
    tx.objectStore(STORE).delete(id);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

function blobToDataUrl(blob: Blob): Promise<string> {
  return new Promise((resolve) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.readAsDataURL(blob);
  });
}

export async function initTournamentAlbum(
  container: HTMLElement,
  tournamentId: string,
  tournamentName: string,
): Promise<void> {
  let stream: MediaStream | null = null;

  async function renderAlbum(): Promise<void> {
    const photos = await getPhotos(tournamentId);

    const thumbsHtml = await Promise.all(
      photos
        .slice()
        .sort((a, b) => a.takenAt.localeCompare(b.takenAt))
        .map(async (p) => {
          const src = await blobToDataUrl(p.blob);
          return `<button class="album-thumb" data-id="${p.id}" style="background-image:url('${src}')"></button>`;
        }),
    );

    container.innerHTML = `
      <div class="page album-page">
        <div class="album-header">
          <h1>${tournamentName}</h1>
        </div>
        <div class="album-grid">
          ${thumbsHtml.length ? thumbsHtml.join("") : `<p class="empty-state album-empty">No photos yet.</p>`}
        </div>
        <button class="album-capture-btn" id="album-capture" aria-label="Take photo">&#128247;</button>
      </div>
    `;

    container.querySelector("#album-capture")!.addEventListener("click", openCamera);

    container.querySelectorAll<HTMLButtonElement>(".album-thumb").forEach((btn) => {
      btn.addEventListener("click", () => openViewer(btn.dataset.id!, photos));
    });
  }

  async function openViewer(photoId: string, photos: Photo[]): Promise<void> {
    const photo = photos.find((p) => p.id === photoId);
    if (!photo) return;
    const src = await blobToDataUrl(photo.blob);
    const date = new Date(photo.takenAt).toLocaleString();

    const overlay = document.createElement("div");
    overlay.className = "album-overlay";
    overlay.innerHTML = `
      <div class="album-viewer">
        <img class="album-viewer-img" src="${src}" alt="Photo" />
        <div class="album-viewer-footer">
          <span class="album-viewer-date">${date}</span>
          <button class="album-viewer-delete" aria-label="Delete photo">&#128465; Delete</button>
        </div>
      </div>
    `;

    const close = () => document.body.removeChild(overlay);
    overlay.addEventListener("click", (e) => { if (e.target === overlay) close(); });

    overlay.querySelector(".album-viewer-delete")!.addEventListener("click", async () => {
      await deletePhoto(photoId);
      close();
      renderAlbum();
    });

    document.body.appendChild(overlay);
  }

  async function openCamera(): Promise<void> {
    // Request max resolution; browser/device will negotiate the best it can
    try {
      stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: "environment", width: { ideal: 3840 }, height: { ideal: 2160 } },
        audio: false,
      });
    } catch {
      try {
        stream = await navigator.mediaDevices.getUserMedia({
          video: { facingMode: "environment" },
          audio: false,
        });
      } catch {
        try {
          stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: false });
        } catch {
          alert("Camera not available.");
          return;
        }
      }
    }

    const track = stream.getVideoTracks()[0];
    const caps = (track.getCapabilities?.() ?? {}) as Record<string, any>;
    const hasHwZoom = caps.zoom !== undefined;
    const zoomMin = caps.zoom?.min ?? 1;
    const zoomMax = hasHwZoom ? Math.min(caps.zoom.max, 8) : 4;
    const zoomStep = caps.zoom?.step ?? 0.5;
    const focusModes: string[] = caps.focusMode ?? [];

    // Enable continuous autofocus if available
    if (focusModes.includes("continuous")) {
      track.applyConstraints({ advanced: [{ focusMode: "continuous" } as any] }).catch(() => {});
    }

    let currentZoom = zoomMin;
    let cssScale = 1;

    const overlay = document.createElement("div");
    overlay.className = "album-overlay";
    overlay.innerHTML = `
      <div class="camera-modal">
        <div class="camera-video-wrap" id="camera-video-wrap">
          <video class="camera-preview" autoplay playsinline muted></video>
          <div class="camera-focus-ring" id="camera-focus-ring"></div>
        </div>
        <div class="camera-zoom-row">
          <button class="camera-zoom-btn" id="camera-zoom-out" aria-label="Zoom out">−</button>
          <span class="camera-zoom-label" id="camera-zoom-label">1×</span>
          <button class="camera-zoom-btn" id="camera-zoom-in" aria-label="Zoom in">+</button>
        </div>
        <div class="camera-controls">
          <button class="camera-cancel" aria-label="Cancel">✕</button>
          <button class="camera-shutter" aria-label="Take photo" disabled></button>
          <div class="camera-spacer"></div>
        </div>
      </div>
    `;

    const video = overlay.querySelector<HTMLVideoElement>(".camera-preview")!;
    const shutter = overlay.querySelector<HTMLButtonElement>(".camera-shutter")!;
    const focusRing = overlay.querySelector<HTMLDivElement>("#camera-focus-ring")!;
    const zoomLabel = overlay.querySelector<HTMLSpanElement>("#camera-zoom-label")!;
    const zoomInBtn = overlay.querySelector<HTMLButtonElement>("#camera-zoom-in")!;
    const zoomOutBtn = overlay.querySelector<HTMLButtonElement>("#camera-zoom-out")!;
    const videoWrap = overlay.querySelector<HTMLDivElement>("#camera-video-wrap")!;

    video.srcObject = stream;
    video.addEventListener("loadedmetadata", () => { shutter.disabled = false; });

    function getZoom(): number { return hasHwZoom ? currentZoom : cssScale; }

    function updateZoom(next: number): void {
      next = Math.max(zoomMin, Math.min(zoomMax, next));
      if (hasHwZoom) {
        currentZoom = next;
        track.applyConstraints({ advanced: [{ zoom: currentZoom } as any] }).catch(() => {});
      } else {
        cssScale = next;
        video.style.transform = `scale(${cssScale})`;
      }
      const z = getZoom();
      zoomLabel.textContent = `${z % 1 === 0 ? z : z.toFixed(1)}×`;
      zoomOutBtn.disabled = z <= zoomMin + 0.01;
      zoomInBtn.disabled = z >= zoomMax - 0.01;
    }

    zoomInBtn.addEventListener("click", () => updateZoom(getZoom() + zoomStep));
    zoomOutBtn.addEventListener("click", () => updateZoom(getZoom() - zoomStep));

    // Pinch-to-zoom
    let pinchDist0 = 0;
    let pinchZoom0 = 1;
    videoWrap.addEventListener("touchstart", (e) => {
      if (e.touches.length === 2) {
        pinchDist0 = Math.hypot(
          e.touches[0].clientX - e.touches[1].clientX,
          e.touches[0].clientY - e.touches[1].clientY,
        );
        pinchZoom0 = getZoom();
      }
    });
    videoWrap.addEventListener("touchmove", (e) => {
      if (e.touches.length === 2) {
        e.preventDefault();
        const d = Math.hypot(
          e.touches[0].clientX - e.touches[1].clientX,
          e.touches[0].clientY - e.touches[1].clientY,
        );
        updateZoom(pinchZoom0 * (d / pinchDist0));
      }
    }, { passive: false });

    // Tap-to-focus
    let lastTouchEnd = 0;
    videoWrap.addEventListener("touchend", (e) => {
      // Ignore the tap that ends a pinch gesture
      if (e.changedTouches.length === 1 && Date.now() - lastTouchEnd > 300) {
        lastTouchEnd = Date.now();
      }
    });
    videoWrap.addEventListener("click", (e) => {
      // Suppress if this was actually the end of a pinch
      if (Date.now() - lastTouchEnd < 400 && lastTouchEnd !== 0) return;

      const rect = videoWrap.getBoundingClientRect();
      const relX = e.clientX - rect.left;
      const relY = e.clientY - rect.top;
      const normX = relX / rect.width;
      const normY = relY / rect.height;

      // Position and show focus ring
      focusRing.style.left = `${relX}px`;
      focusRing.style.top = `${relY}px`;
      focusRing.className = "camera-focus-ring focusing";

      const onFocusDone = (success: boolean) => {
        focusRing.className = `camera-focus-ring ${success ? "focused" : ""}`;
        setTimeout(() => { focusRing.className = "camera-focus-ring"; }, 900);
      };

      if (focusModes.includes("single-shot") || focusModes.includes("manual")) {
        const mode = focusModes.includes("single-shot") ? "single-shot" : "manual";
        track.applyConstraints({
          advanced: [{ focusMode: mode, pointsOfInterest: [{ x: normX, y: normY }] } as any],
        }).then(() => onFocusDone(true)).catch(() => onFocusDone(false));
      } else {
        // Visual feedback only
        setTimeout(() => onFocusDone(true), 350);
      }
    });

    const stopStream = () => {
      stream?.getTracks().forEach((t) => t.stop());
      stream = null;
    };
    const close = () => { stopStream(); document.body.removeChild(overlay); };

    overlay.querySelector(".camera-cancel")!.addEventListener("click", close);

    shutter.addEventListener("click", async () => {
      shutter.disabled = true;
      const w = video.videoWidth || 1280;
      const h = video.videoHeight || 720;
      const canvas = document.createElement("canvas");

      if (hasHwZoom || cssScale <= 1) {
        // Full sensor resolution
        canvas.width = w;
        canvas.height = h;
        canvas.getContext("2d")!.drawImage(video, 0, 0, w, h);
      } else {
        // Crop to match the CSS-scaled view
        const srcW = w / cssScale;
        const srcH = h / cssScale;
        const srcX = (w - srcW) / 2;
        const srcY = (h - srcH) / 2;
        canvas.width = Math.round(srcW);
        canvas.height = Math.round(srcH);
        canvas.getContext("2d")!.drawImage(video, srcX, srcY, srcW, srcH, 0, 0, canvas.width, canvas.height);
      }

      let blob: Blob | null = await new Promise<Blob | null>((resolve) =>
        canvas.toBlob(resolve, "image/jpeg", 0.92),
      );
      if (!blob) {
        blob = await fetch(canvas.toDataURL("image/jpeg", 0.92)).then((r) => r.blob());
      }
      if (!blob) { shutter.disabled = false; return; }

      const takenAt = new Date().toISOString();
      const filename = `${tournamentName.replace(/[^a-z0-9]/gi, "_")}_${takenAt.replace(/[:.]/g, "-")}.jpg`;
      const data = await blobToBase64(blob);
      invoke("save_photo_to_gallery", { album: "TheJudgeApp", filename, data }).catch((err) => {
        console.error("Failed to save photo to gallery:", err);
      });
      await savePhoto({ id: crypto.randomUUID(), tournamentId, blob, takenAt });
      close();
      renderAlbum();
    });

    document.body.appendChild(overlay);
    updateZoom(zoomMin);
  }

  await renderAlbum();
}
