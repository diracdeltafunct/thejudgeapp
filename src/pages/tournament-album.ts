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
    try {
      stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: "environment" },
        audio: false,
      });
    } catch {
      // Fall back to any camera if rear-facing not available
      try {
        stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: false });
      } catch {
        alert("Camera not available.");
        return;
      }
    }

    const overlay = document.createElement("div");
    overlay.className = "album-overlay";
    overlay.innerHTML = `
      <div class="camera-modal">
        <video class="camera-preview" autoplay playsinline></video>
        <div class="camera-controls">
          <button class="camera-cancel" aria-label="Cancel">✕</button>
          <button class="camera-shutter" aria-label="Take photo"></button>
          <div class="camera-spacer"></div>
        </div>
      </div>
    `;

    const video = overlay.querySelector<HTMLVideoElement>(".camera-preview")!;
    const shutter = overlay.querySelector<HTMLButtonElement>(".camera-shutter")!;
    shutter.disabled = true;
    video.srcObject = stream;

    // Enable shutter only once the video stream has real dimensions
    video.addEventListener("loadedmetadata", () => {
      shutter.disabled = false;
    });

    const stopStream = () => {
      stream?.getTracks().forEach((t) => t.stop());
      stream = null;
    };

    const close = () => {
      stopStream();
      document.body.removeChild(overlay);
    };

    overlay.querySelector(".camera-cancel")!.addEventListener("click", close);

    shutter.addEventListener("click", async () => {
      shutter.disabled = true;

      const w = video.videoWidth || 1280;
      const h = video.videoHeight || 720;
      const canvas = document.createElement("canvas");
      canvas.width = w;
      canvas.height = h;
      const ctx = canvas.getContext("2d");
      if (!ctx) { shutter.disabled = false; return; }
      ctx.drawImage(video, 0, 0, w, h);

      // toBlob is preferred; fall back to toDataURL→fetch on Android WebView
      let blob: Blob | null = await new Promise<Blob | null>((resolve) =>
        canvas.toBlob(resolve, "image/jpeg", 0.85),
      );
      if (!blob) {
        const dataUrl = canvas.toDataURL("image/jpeg", 0.85);
        blob = await fetch(dataUrl).then((r) => r.blob());
      }
      if (!blob) { shutter.disabled = false; return; }

      const takenAt = new Date().toISOString();
      const filename = `${tournamentName.replace(/[^a-z0-9]/gi, "_")}_${takenAt.replace(/[:.]/g, "-")}.jpg`;

      const data = await blobToBase64(blob);
      await invoke("save_photo_to_gallery", { album: "TheJudgeApp", filename, data });
      await savePhoto({ id: crypto.randomUUID(), tournamentId, blob, takenAt });
      close();
      renderAlbum();
    });

    document.body.appendChild(overlay);
  }

  await renderAlbum();
}
