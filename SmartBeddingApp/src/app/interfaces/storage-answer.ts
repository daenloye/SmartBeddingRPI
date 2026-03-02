import { StorageFolder } from "./storage-folder";
import { StorageSystem } from "./storage-system";

export interface StorageAnswer {
  registers:StorageFolder[];
  system:StorageSystem;
}
