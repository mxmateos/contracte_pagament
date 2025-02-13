// FUNIONAMENT GENERAL DEL CONTRACTE
// L'estudiant s'inscriu al curs pagant el curs.
// El professor rep el pagament proporcional de cada classe un poc s'ha completat la classe i l'alumne ho confirma.
// El curs passa de l'estat Ongoing a Completed quan totes les clases s'han fet.

#![no_std]

use multiversx_sc::derive_imports::*;
use multiversx_sc::imports::*;

#[derive(TopEncode, TopDecode, TypeAbi, PartialEq, Clone, Copy)]
pub enum CourseStatus {
    Ongoing,
    Completed,
}


#[multiversx_sc::contract]
pub trait PagoCurso {
    #[init]
    fn init(&self, teacher: ManagedAddress, course_fee: BigUint, total_classes: u64) {

        // Només el owner, que es el teacher, pot inicialitzar el contracte.
        let contract_owner = self.blockchain().get_owner_address();
        let caller = self.blockchain().get_caller();
        require!(caller == contract_owner, "Only the contract owner can initialize the contract");

        require!(course_fee > 0, "Course fee must be more than 0");
        require!(total_classes > 0, "Total classes must be more than 0");

        self.teacher().set(teacher);
        self.course_fee().set(course_fee);
        self.total_classes().set(total_classes);
        self.classes_completed().set(0);
        self.course_status().set(CourseStatus::Ongoing);
    }

    #[upgrade]
    fn upgrade(&self) {}

    // l'estudiant fa el pagament, i se l'inscriu al array d'estudiants.
    #[endpoint]
    #[payable("EGLD")]
    fn enroll(&self) {
        let payment = self.call_value().egld().clone_value();
        let course_fee = self.course_fee().get();

        require!(payment == course_fee, "Incorrect payment amount");
        require!(self.course_status().get() == CourseStatus::Ongoing,"Course is already completed");
        let caller = self.blockchain().get_caller();
        require!(!self.students().contains_key(&caller), "Student is already enrolled");
        // Afegeix el pagament a l'array d'estudiants.
        self.students().insert(caller.clone(), payment);

    }

    // cada cop que l'estudiant fa una classe signa
    #[endpoint]
    fn complete_class(&self) {
        let caller = self.blockchain().get_caller();
        require!(!self.students().contains_key(&caller), "Only enrolled students can complete classes");
        require!(self.course_status().get() == CourseStatus::Ongoing, "Course is already completed");

        // Calcula la part proporcional que s'envia al proferssor
        let proportional_payment = self.calculate_proportional_payment();
        let teacher = self.teacher().get();
        self.send().direct_egld(&teacher, &proportional_payment);

        // modifica la cuantitat que li resta a l'estudiant del que ha pagat'
        let mut student_payment = self.students().get(&caller).unwrap_or_else(|| BigUint::zero());
        student_payment -= proportional_payment;
        self.students().insert(caller.clone(), student_payment);
    }

    //El professor signa cada classe que fa per tenir el recompte de totes les classes fetes, això simplifica el contracte.
    // per poder comptar el número real de classes fetes. I que això no depengui d'un o diversos estudiants.
    #[endpoint]
    fn sign_class(&self) {
        let caller = self.blockchain().get_caller();
        let teacher = self.teacher().get();

        require!(caller == teacher, "Only the teacher can confirm the real class completion");

        // Incrementa el comptador de les classes
        let mut classes_completed = self.classes_completed().get();
        classes_completed += 1;
        self.classes_completed().set(classes_completed);

        // Mira si el curs està completat, i en aquest cas crida la funció refund.
        if classes_completed >= self.total_classes().get() {
            self.course_status().set(CourseStatus::Completed);
            self.refund_remaining_funds();
        }
    }

    fn refund_remaining_funds(&self) {
        for (student_address, student_payment) in self.students().iter() {
            if student_payment > BigUint::zero() {
                self.send().direct_egld(&student_address, &student_payment);
                self.students().insert(student_address.clone(), BigUint::zero());
            }
        }
    }




    #[view(calculateProportionalPayment)]
    fn calculate_proportional_payment(&self) -> BigUint {
        let course_fee = self.course_fee().get();
        let total_classes = self.total_classes().get();
        course_fee / total_classes
    }

    #[view(getCurrentFunds)]
    fn get_current_funds(&self) -> BigUint {
        self.blockchain()
            .get_sc_balance(&EgldOrEsdtTokenIdentifier::egld(), 0)
    }

    // Memoria blockchain

    #[view(getTeacher)]
    #[storage_mapper("teacher")]
    fn teacher(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getStudents)]
    #[storage_mapper("students")]
    fn students(&self) -> MapMapper<ManagedAddress, BigUint>;

    #[view(getCourseFee)]
    #[storage_mapper("course_fee")]
    fn course_fee(&self) -> SingleValueMapper<BigUint>;

    #[view(getTotalClasses)]
    #[storage_mapper("total_classes")]
    fn total_classes(&self) -> SingleValueMapper<u64>;

    #[view(getClassesCompleted)]
    #[storage_mapper("classes_completed")]
    fn classes_completed(&self) -> SingleValueMapper<u64>;

    #[view(getCourseStatus)]
    #[storage_mapper("course_status")]
    fn course_status(&self) -> SingleValueMapper<CourseStatus>;

}


// MES DETALLS SOBRE LA LLÒGICA.
//4. Exemple
//Pagament del curs :
//Diferents estudiants s'insciruen pagan el curs.
//Cada estudiant s'afegeix a un array d'estudiants amb la cuantita de classes completes
//Classe completada :
//Quan el professor completa una classe, els dos signen.
//Es fan els pagaments proporcionals al professor.
//Curs completat :
//Una vegada totes les clases s'han completat, l'estatus del curs canvia a completat.
//Els diners restants es retornen als estudiants.
