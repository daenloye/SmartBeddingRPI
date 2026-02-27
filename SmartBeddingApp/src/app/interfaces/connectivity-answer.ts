import { ConnectivityNetwork } from "./connectivity-network";

export interface ConnectivityAnswer {
  APMode:boolean;
  BrokerMQTT:boolean;
  WifiSSID:string;
  Networks:ConnectivityNetwork[];
}
