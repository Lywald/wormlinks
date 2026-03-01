<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke, convertFileSrc } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  
  interface WormlinkMatch {
    url: string;
    source_app: string;
    is_demo: boolean;
  }

  const appWindow = getCurrentWindow();

  // Core reactive state
  let linkUrl = $state('');
  let sourceApp = $state('');
  let isDemo = $state(false);
  let widgetType = $state('');
  
  // Specific widget states
  let imageUrl = $state('');
  let youtubeId = $state('');
  let webUrl = $state('');
  let mapQuery = $state('');
  let pdfPath = $state('');
  
  let isMaximized = $state(false);
  let isPinned = $state(false);
  let isHovered = $state(false);
  let updateId = $state(0);

  // DIRECT INJECTION HOOK
  (window as any).updatePortalContent = (url: string, app: string, demo: boolean) => {
    if (url === linkUrl && !isMaximized) return;

    linkUrl = url;
    sourceApp = app;
    isDemo = demo;
    
    try {
      const parseable = url.replace('wormlink://', 'http://');
      const parsed = new URL(parseable);
      widgetType = parsed.host; 
      
      // Reset all type-specific states
      imageUrl = '';
      youtubeId = '';
      webUrl = '';
      mapQuery = '';
      pdfPath = '';

      const params = parsed.searchParams;

      if (widgetType === 'image') {
        imageUrl = params.get('id') || 'https://images.unsplash.com/photo-1550745165-9bc0b252726f';
      } else if (widgetType === 'youtube') {
        youtubeId = params.get('id') || '';
      } else if (widgetType === 'web') {
        webUrl = params.get('url') || '';
      } else if (widgetType === 'map') {
        mapQuery = params.get('q') || '';
      } else if (widgetType === 'pdf') {
        const rawPath = params.get('path') || '';
        if (rawPath) {
          // In Windows, paths like C:\ must be handled carefully
          pdfPath = convertFileSrc(rawPath);
          console.log('PDF Asset URL:', pdfPath);
        }
      }
      
      updateId += 1;
    } catch (e) {
      console.error('Portal parse failed:', e);
    }
  };

  (window as any).updateMaximizeState = (max: boolean) => {
    isMaximized = max;
  };

  async function closePortal() {
    isPinned = false;
    await invoke('dismiss_portal');
  }

  async function toggleMax() {
    await invoke('toggle_maximize');
  }

  async function togglePin() {
    isPinned = !isPinned;
    const backendState = await invoke<boolean>('toggle_pin');
    isPinned = backendState;
  }

  onMount(async () => {
    try {
      const initial = await invoke<WormlinkMatch | null>('get_wormlink_data');
      if (initial) (window as any).updatePortalContent(initial.url, initial.source_app, initial.is_demo);
    } catch (e) { console.error('Initial sync failed', e); }
  });
</script>

<div 
  class="w-screen h-screen flex items-center justify-center bg-transparent transition-opacity duration-300"
  onclick={(e) => { if(isMaximized && e.target === e.currentTarget) toggleMax(); }}
  onmouseenter={() => isHovered = true}
  onmouseleave={() => isHovered = false}
  role="presentation"
>
  {#if linkUrl}
    <main 
      class={`flex flex-col bg-black/95 text-white rounded-2xl shadow-2xl border border-white/30 p-4 backdrop-blur-3xl relative overflow-hidden ring-1 ring-white/10 group/main transition-all duration-500 ease-out ${isMaximized ? 'w-[80%] h-[80%]' : 'w-full h-full'} ${isPinned ? 'ring-cyan-500/50 shadow-[0_0_30px_rgba(6,182,212,0.2)]' : ''}`}
    >
      <!-- Top bar -->
      <div class="absolute top-0 left-0 right-0 h-10 flex items-center justify-between px-4 z-50">
        <div data-tauri-drag-region class="flex-1 h-full cursor-move"></div>
        <div class="flex items-center space-x-3">
          <button onclick={togglePin} class="p-1.5 hover:bg-white/10 rounded-lg transition-all cursor-pointer flex items-center justify-center" title="Pin Portal">
            <span class={`text-[12px] leading-none transition-transform duration-300 ${!isPinned ? 'rotate-45 text-gray-500' : 'scale-125 text-cyan-400'}`}>
              📌
            </span>
          </button>
          <button onclick={toggleMax} class="p-1.5 hover:bg-white/10 rounded-lg transition-all cursor-pointer font-bold text-gray-400 text-[10px]" title="Maximize">
            {isMaximized ? '❐' : '□'}
          </button>
          <button onclick={closePortal} class="p-1.5 hover:bg-red-500/20 hover:text-red-400 rounded-lg transition-all cursor-pointer group/close" title="Close">
            <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 text-gray-500 group-hover/close:text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>
      
      <div class="flex flex-col w-full h-full mt-4">
        <div class="flex items-start justify-between mb-3 w-full space-x-4">
          <div class="flex flex-col flex-shrink-0">
            <div class="flex items-center space-x-2">
              <div class={`w-1.5 h-1.5 rounded-full animate-pulse ${
                widgetType === 'youtube' ? 'bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.8)]' : 
                widgetType === 'web' ? 'bg-blue-500 shadow-[0_0_8px_rgba(59,130,246,0.8)]' : 
                widgetType === 'map' ? 'bg-green-500 shadow-[0_0_8px_rgba(34,197,94,0.8)]' :
                widgetType === 'pdf' ? 'bg-orange-500 shadow-[0_0_8px_rgba(249,115,22,0.8)]' :
                'bg-cyan-400 shadow-[0_0_8px_rgba(34,211,238,0.8)]'
              }`}></div>
              <span class={`text-[9px] font-bold tracking-[0.15em] uppercase ${
                widgetType === 'youtube' ? 'text-red-400' : 
                widgetType === 'web' ? 'text-blue-400' : 
                widgetType === 'map' ? 'text-green-400' :
                widgetType === 'pdf' ? 'text-orange-400' :
                'text-cyan-300'
              }`}>Wormlink Portal</span>
              {#if isPinned}
                <span class="text-[7px] font-black bg-cyan-500/20 text-cyan-400 px-1 rounded border border-cyan-500/30 ml-1">LOCKED</span>
              {/if}
            </div>
            <div class="text-[10px] font-semibold text-gray-400 mt-0.5 ml-3.5">
              via <span class="text-white/90">{sourceApp}</span>
            </div>
          </div>
          <div class="flex-1 flex justify-end overflow-hidden pt-1">
            <div class="text-[9px] font-mono font-medium text-gray-500 bg-white/5 px-2.5 py-1 rounded-full border border-white/5 truncate max-w-[120px] transition-all duration-300 group-hover/main:max-w-full group-hover/main:text-white">
              {linkUrl}
            </div>
          </div>
        </div>

        <div class="flex-1 w-full bg-black/40 rounded-xl overflow-hidden flex items-center justify-center border border-white/10 relative shadow-inner">
          {#if widgetType === 'image'}
            <img src={imageUrl} 
                 alt="Wormlink widget" 
                 class="max-w-full max-h-full object-contain transition-all duration-500 group-hover:scale-105 pointer-events-none" />
            <div class="absolute inset-0 bg-gradient-to-t from-cyan-500/5 to-transparent pointer-events-none"></div>
          {:else if widgetType === 'youtube' && youtubeId}
            <iframe
              class="w-full h-full border-0"
              src={`https://www.youtube.com/embed/${youtubeId}?autoplay=1&mute=0&controls=1&rel=0`}
              title="YouTube video player"
              allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
              allowfullscreen
            ></iframe>
          {:else if widgetType === 'web' && webUrl}
            <div class="w-full h-full flex flex-col">
              <iframe
                class="flex-1 w-full border-0 bg-white"
                src={`wormhole://localhost/${encodeURIComponent(webUrl)}`}
                title="Web Portal"
                allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                allowfullscreen
              ></iframe>
              <div class="bg-gray-900 px-3 py-1 text-[8px] text-gray-400 border-t border-white/10 flex justify-between">
                <span>Optimized via Wormhole Proxy</span>
                <a href={webUrl} target="_blank" class="text-blue-400 hover:underline">Open in Browser</a>
              </div>
            </div>
          {:else if widgetType === 'map' && mapQuery}
            <iframe
              class="w-full h-full border-0"
              src={`https://www.google.com/maps?q=${encodeURIComponent(mapQuery)}&output=embed`}
              title="Map Portal"
              allowfullscreen
            ></iframe>
          {:else if widgetType === 'pdf' && pdfPath}
            <iframe
              class="w-full h-full bg-white rounded-lg border-0"
              src={pdfPath}
              title="PDF Viewer"
            ></iframe>
          {:else}
            <div class="flex flex-col items-center justify-center text-center space-y-3 p-6">
               <div class="p-3 rounded-full bg-white/5 border border-white/10">
                 <svg xmlns="http://www.w3.org/2000/svg" class="w-6 h-6 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                   <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                 </svg>
               </div>
               <div class="space-y-1">
                 <h2 class="text-xs font-bold text-white tracking-wide uppercase">Portal Active</h2>
                 <p class="text-[10px] text-gray-400 font-medium">Type: <span class="text-white/80">{widgetType || 'Unknown'}</span></p>
               </div>
            </div>
          {/if}
        </div>
      </div>
    </main>
  {/if}
</div>

<style>
  :global(body) { background: transparent !important; }
</style>
