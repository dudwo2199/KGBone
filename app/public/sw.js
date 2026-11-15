// KGBone 서비스 워커: 설치한 웹 앱이 인터넷 없이도 열리게 화면 파일을 캐시에 담아 둔다.
// 온라인이면 항상 서버 것을 먼저 쓰고(새 판이 바로 보임), 서버에 닿지 않을 때만 캐시를 쓴다.
// 아래 두 줄은 빌드 때 tools/vite-precache.mjs가 실제 값으로 바꾼다.
const VERSION = 'dev';
const FILES = [];

const PREFIX = 'kgbone-';
const CACHE = PREFIX + VERSION;

self.addEventListener('install', (e) => {
  e.waitUntil(
    caches
      .open(CACHE)
      .then((c) => c.addAll(FILES))
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener('activate', (e) => {
  e.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith(PREFIX) && k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

async function fromCache(req) {
  const hit = await caches.match(req, { ignoreSearch: true });
  if (hit) return hit;
  if (req.mode === 'navigate') {
    const page = await caches.match('./');
    if (page) return page;
  }
  return Response.error();
}

self.addEventListener('fetch', (e) => {
  const req = e.request;
  if (req.method !== 'GET' || new URL(req.url).origin !== self.location.origin) return;
  e.respondWith(
    fetch(req)
      .then((res) => {
        if (res.ok) {
          const copy = res.clone();
          e.waitUntil(caches.open(CACHE).then((c) => c.put(req, copy)));
        }
        return res;
      })
      .catch(() => fromCache(req)),
  );
});
