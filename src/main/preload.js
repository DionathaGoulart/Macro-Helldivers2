const { contextBridge, ipcRenderer } = require('electron')

contextBridge.exposeInMainWorld('api', {
  // Funções genéricas
  send: (channel, data) => ipcRenderer.send(channel, data),
  invoke: (channel, ...args) => ipcRenderer.invoke(channel, ...args),
  
  // Funções de Escuta (Main -> Renderer)
  onUpdateStatus: (callback) => ipcRenderer.on('update-status', (event, info) => callback(info)),
  onGameFocusChanged: (callback) => ipcRenderer.on('game-focus-changed', (event, focused) => callback(focused)),
  onMacroTriggered: (callback) => ipcRenderer.on('macro-triggered', (event, index) => callback(index)),
  onToggleMinimalMode: (callback) => ipcRenderer.on('toggle-minimal-mode', (event, isMinimal) => callback(isMinimal)),
  onSyncSlots: (callback) => ipcRenderer.on('sync-slots', (event, slots) => callback(slots)),
  onSyncSettings: (callback) => ipcRenderer.on('sync-settings', (event, settings) => callback(settings)),

  // Funções de Envio (Renderer -> Main)
  updateSlots: (slots) => ipcRenderer.send('update-slots', slots),
  saveSettings: (settings) => ipcRenderer.send('save-settings', settings),
  hideWindow: () => ipcRenderer.send('hide-window'),
  hideOverlay: () => ipcRenderer.send('hide-overlay')
})
