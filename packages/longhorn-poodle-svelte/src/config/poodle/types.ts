import type {
  ConfigOperationsClient,
  ConfigOperationsSnapshot,
} from "@inflatable-cookie/longhorn/config";

export interface ConfigOperationsPageProps {
  client: ConfigOperationsClient;
  initialSnapshot?: ConfigOperationsSnapshot | null;
  nextRequestId?: () => string;
  onSnapshot?: (snapshot: ConfigOperationsSnapshot) => void;
}
