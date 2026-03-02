import { StorageFile } from "./storage-file";

export interface StorageFolder {
  created: string;
  jsonFiles:StorageFile[];
  jsonUsedMb: number;
  name: string;
  path: string;
  totalUsedMb: number;
  wavFiles:StorageFile[];
  wavUsedMb: number;
}
