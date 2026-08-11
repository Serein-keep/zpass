import type { Component } from "vue";
import {
  GlobeOutline,
  ServerOutline,
  MailOutline,
  PhonePortraitOutline,
  WifiOutline,
  KeyOutline,
  LockClosedOutline,
  CardOutline,
  FolderOpenOutline,
  DocumentTextOutline,
  ShieldCheckmarkOutline,
  CloudOutline,
  HomeOutline,
  PeopleOutline,
  TimeOutline,
  AlbumsOutline,
  BookOutline,
  PersonOutline,
  GitBranchOutline,
  HardwareChipOutline,
} from "@vicons/ionicons5";

const ICON_MAP: Record<string, Component> = {
  globe: GlobeOutline,
  server: ServerOutline,
  mail: MailOutline,
  "phone-portrait": PhonePortraitOutline,
  wifi: WifiOutline,
  key: KeyOutline,
  "lock-closed": LockClosedOutline,
  card: CardOutline,
  "folder-open": FolderOpenOutline,
  "document-text": DocumentTextOutline,
  "shield-checkmark": ShieldCheckmarkOutline,
  cloud: CloudOutline,
  home: HomeOutline,
  people: PeopleOutline,
  time: TimeOutline,
  albums: AlbumsOutline,
  book: BookOutline,
  person: PersonOutline,
  "git-branch": GitBranchOutline,
  "hardware-chip": HardwareChipOutline,
};

export function templateIcon(key?: string | null): Component {
  return (key && ICON_MAP[key]) || FolderOpenOutline;
}
