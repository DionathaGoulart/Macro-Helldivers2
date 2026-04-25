const { contextBridge, ipcRenderer } = require('electron')

contextBridge.exposeInMainWorld('api', {
  // Funções genéricas
  send: (channel, data) => ipcRenderer.send(channel, data),
  invoke: (channel, ...args) => ipcRenderer.invoke(channel, ...args),
  
  // Funções específicas que o App.jsx espera
  updateSlots: (slots) => ipcRenderer.send('update-slots', slots),
  saveSettings: (settings) => ipcRenderer.send('save-settings', settings),
  onMacroTriggered: (callback) => ipcRenderer.on('macro-triggered', (event, index) => callback(index))
})
