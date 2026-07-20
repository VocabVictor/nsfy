export interface Message {
  id: string;
  time: number;
  title: string;
  message: string;
  priority: number;
  tags: string[];
  category: string[];
  popup: boolean;
  bypassDnd: boolean;
  read: boolean;
}

export interface Topic {
  name: string;
  server: string;
  messages: Message[];
  unread: number;
  connected: boolean;
  lastConnectedAt?: number;
}

export interface Server {
  url: string;
  name: string;
  token?: string;
}
