export interface ApiResponse<T> {
  result: boolean;
  timestamp: string;
  data: T | null;
  message: string | null;
}
