import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ApiService } from '../../services/api.service';
import { ConnectivityAnswer } from '../../interfaces/connectivity-answer';
import { timer, Subscription, of } from 'rxjs';
import { switchMap, catchError } from 'rxjs/operators';
import { FormsModule } from '@angular/forms';
import { AlertService } from '../../services/alert.service';

@Component({
  selector: 'app-connectivity',
  standalone: true,
  imports: [CommonModule,FormsModule], // Puedes quitarlo si no usas pipes como | async o | date
  templateUrl: './connectivity.component.html',
  styleUrl: './connectivity.component.css'
})
export class ConnectivityComponent implements OnInit, OnDestroy {

  data?: ConnectivityAnswer;
  loading: boolean = true;
  refreshing: boolean = false;
  private refreshSub?: Subscription;

  //Rutas
  SSID: string = "";
  password: string = ""

  selectedRegister: any = null;

  constructor(private apiService: ApiService, private alertService:AlertService) { }

  ngOnInit(): void {
    this.refreshSub = timer(0, 8000).pipe(
      switchMap(() => {
        this.refreshing = true;
        return this.apiService.getConnectivity().pipe(
          catchError(err => {
            console.error("Error en polling:", err);
            return of(null);
          })
        );
      })
    ).subscribe({
      next: (response) => {
        if (response?.result && response.data) {
          this.data = response.data;
          console.log("Data de conectividad:", this.data);
        }
        this.loading = false;
        this.refreshing = false;
      }
    });
  }

  ngOnDestroy(): void {
    this.refreshSub?.unsubscribe();
  }

  openModal(Index: number) {
    if (!this.data) return;

    this.selectedRegister = this.data.Networks[Index];

    this.SSID= this.selectedRegister.SSID;
    this.password = "";

    console.log("Index del folder seleccionado:", Index);
    const modal = document.getElementById('connectionModal') as HTMLDialogElement;
    if (modal) {
      modal.showModal();
    }
  }

  conectar(){
    this.apiService.connectWifi(this.SSID, this.password).subscribe({
      next: (response) => {
        if (response?.result && response.data) {
          this.alertService.addAlert({
            message: response.message || 'Se han eviado las credenciales, intentando conectar...',
            type: 'success',
            duration: 3000
          });
        }else{
          this.alertService.addAlert({
            message: response.message || 'Error al enviar credenciales de red',
            type: 'error',
            duration: 3000
          });
        }

      },
      error: (err) => {
        this.alertService.addAlert({
          message: 'Error al enviar credenciales de red',
          type: 'error',
          duration: 3000
        });
      },
      complete: () => {
      }
    });
  }


}
