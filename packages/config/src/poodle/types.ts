import type {
  ConfigOperationsClient,
  ConfigOperationsSnapshot,
} from "../index.ts";

export interface ConfigOperationsPageProps {
  client: ConfigOperationsClient;
  initialSnapshot?: ConfigOperationsSnapshot | null;
  nextRequestId?: () => string;
  onSnapshot?: (snapshot: ConfigOperationsSnapshot) => void;
}
