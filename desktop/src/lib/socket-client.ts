import NativeWebSocket from '@tauri-apps/plugin-websocket';

export interface SocketConnection {
  close(): void;
}

interface SocketHandlers {
  open: () => void;
  message: (text: string) => void;
  close: () => void;
  error?: () => void;
}

export async function connectSocket(
  url: string,
  direct: boolean,
  token: string | undefined,
  handlers: SocketHandlers,
): Promise<SocketConnection> {
  if (!direct) return connectBrowser(url, token, handlers);
  try {
    const socket = await NativeWebSocket.connect(url, {
      headers: token ? { Authorization: `Bearer ${token}` } : undefined,
      readBufferSize: 4096,
      writeBufferSize: 4096,
      maxMessageSize: 1_100_000,
      maxFrameSize: 1_100_000,
    });
    let closed = false;
    const remove = socket.addListener(message => {
      if (message.type === 'Text') handlers.message(message.data);
      if (message.type === 'Close' && !closed) {
        closed = true;
        handlers.close();
      }
    });
    handlers.open();
    return {
      close() {
        if (closed) return;
        closed = true;
        remove();
        void socket.disconnect();
      },
    };
  } catch (error) {
    handlers.error?.();
    handlers.close();
    throw error;
  }
}

function connectBrowser(
  url: string,
  token: string | undefined,
  handlers: SocketHandlers,
): SocketConnection {
  const socket = new WebSocket(url);
  socket.onopen = () => {
    if (token) socket.send(JSON.stringify({ type: 'auth', token }));
    handlers.open();
  };
  socket.onmessage = event => handlers.message(String(event.data));
  socket.onclose = handlers.close;
  socket.onerror = handlers.error || null;
  return { close: () => socket.close() };
}
