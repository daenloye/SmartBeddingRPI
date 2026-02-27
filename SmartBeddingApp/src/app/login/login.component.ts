import { ApiService } from './../services/api.service';
import { FormsModule } from '@angular/forms';
import { Component } from '@angular/core';
import { AlertService } from '../services/alert.service';

@Component({
  selector: 'app-login',
  imports: [FormsModule],
  templateUrl: './login.component.html',
  styleUrl: './login.component.css'
})
export class LoginComponent {

  constructor(
    private apiService: ApiService,
    private alertService: AlertService
  ) { }

  code:number|null = null;

  onLogin() {
    if (this.code === null) {
      //Envio el mensaje
      this.alertService.addAlert({
        message: 'Por favor ingresa un código válido.'  ,
        type: 'info',
        duration: 3000
      });
      return;
    }

    this.apiService.login(this.code.toString()).subscribe({
      next: (res) => {
        console.log('Login response:', res);
        if (res.result) {
          this.alertService.addAlert({
            message: '¡Inicio de sesión exitoso!',
            type: 'success',
            duration: 3000
          });
        } else {
          this.alertService.addAlert({
            message: res.message || 'Error desconocido.',
            type: 'error',
            duration: 3000
          });
        }
      },
      error: (err) => {
        console.error('Login error:', err);
        this.alertService.addAlert({
          message: 'No se pudo conectar al servidor.',
          type: 'error',
          duration: 3000
        });
      }
    });
  }
}
