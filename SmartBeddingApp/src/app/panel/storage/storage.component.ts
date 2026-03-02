import { Component } from '@angular/core';
import { ApiService } from '../../services/api.service';
import { StorageAnswer } from '../../interfaces/storage-answer';
import { DecimalPipe } from '@angular/common';
import { StorageFolder } from '../../interfaces/storage-folder';

@Component({
  selector: 'app-storage',
  imports: [DecimalPipe],
  templateUrl: './storage.component.html',
  styleUrl: './storage.component.css'
})
export class StorageComponent {

  protected Math = Math;

  data?: StorageAnswer;
  loading: boolean = true;
  refreshing: boolean = false;

  selectedRegister:StorageFolder | null = null;

  constructor(private apiService: ApiService) { }

  ngOnInit(): void {
    this.loadData();
  }

  loadData(){
    this.refreshing = true;

    this.apiService.getStorage().subscribe({
      next: (response) => {
        if (response?.result && response.data) {
          const rawData = response.data as StorageAnswer;

          // Ordenamos cada carpeta de registros
          rawData.registers.forEach(folder => {
            // Ordenar JSONs
            folder.jsonFiles.sort((a, b) =>
              a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' })
            );

            // Ordenar WAVs
            folder.wavFiles.sort((a, b) =>
              a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' })
            );
          });

          this.data = rawData;
        }
        this.loading = false;
        this.refreshing = false;

        console.log("Data de almacenamiento:", this.data);
      },
      error: (err) => {
        console.error("Error en polling:", err);
        this.loading = false;
        this.refreshing = false;
      }
    });
  }

  openModal(folderIndex: number) {
    if (!this.data) return;

    this.selectedRegister = this.data.registers[folderIndex];

    console.log("Index del folder seleccionado:", folderIndex);
    const modal = document.getElementById('informationModal') as HTMLDialogElement;
    if (modal) {
      modal.showModal();
    }
  }

  // Suma de todos los totalUsedMb de los registros
  getTotalRegistersMb(): number {
    return this.data?.registers.reduce((acc, reg) => acc + reg.totalUsedMb, 0) || 0;
  }

  // % de la SD que ocupan tus registros
  getRegistersPercentage(): number {
    if (!this.data) return 0;
    return (this.getTotalRegistersMb() / this.data.system.diskTotalMb) * 100;
  }

  // % total ocupado (Total - Libre)
  getUsedPercentage(): number {
    if (!this.data) return 0;
    const used = this.data.system.diskTotalMb - this.data.system.diskFreeMb;
    return (used / this.data.system.diskTotalMb) * 100;
  }

  // % de "Otros" (Lo ocupado total menos lo que sabemos que son registros)
  getSystemOtherPercentage(): number {
    const totalUsed = this.getUsedPercentage();
    const regUsed = this.getRegistersPercentage();
    return Math.max(0, totalUsed - regUsed);
  }

}
