// Pattern 1.2: Parent Observing Child State - Form Validation Summary
// Parent observes validation state from all form sections and shows summary

import { Component, state, setState } from '@minimact/core';

// Child component: PersonalInfoForm
export function PersonalInfoForm() {
  const isValid = state.isValid;
  const data = state.data || {};

  const validate = (field: string, value: string) => {
    const newData = { ...data, [field]: value };
    const valid = newData.name && newData.email && newData.phone;
    setState('isValid', valid);
    setState('data', newData);
  };

  return (
    <div className={`form-section ${isValid ? 'valid' : 'invalid'}`}>
      <h2>Personal Information</h2>
      <input
        id="name-input"
        type="text"
        placeholder="Name"
        value={data.name || ''}
        onInput={(e) => validate('name', e.target.value)}
      />
      <input
        id="email-input"
        type="email"
        placeholder="Email"
        value={data.email || ''}
        onInput={(e) => validate('email', e.target.value)}
      />
      <input
        id="phone-input"
        type="tel"
        placeholder="Phone"
        value={data.phone || ''}
        onInput={(e) => validate('phone', e.target.value)}
      />
      {isValid && <span className="check-mark">✓</span>}
    </div>
  );
}

// Child component: AddressForm
export function AddressForm() {
  const isValid = state.isValid;
  const data = state.data || {};

  const validate = (field: string, value: string) => {
    const newData = { ...data, [field]: value };
    const valid = newData.street && newData.city && newData.zip;
    setState('isValid', valid);
    setState('data', newData);
  };

  return (
    <div className={`form-section ${isValid ? 'valid' : 'invalid'}`}>
      <h2>Address</h2>
      <input
        id="street-input"
        type="text"
        placeholder="Street"
        value={data.street || ''}
        onInput={(e) => validate('street', e.target.value)}
      />
      <input
        id="city-input"
        type="text"
        placeholder="City"
        value={data.city || ''}
        onInput={(e) => validate('city', e.target.value)}
      />
      <input
        id="zip-input"
        type="text"
        placeholder="ZIP Code"
        value={data.zip || ''}
        onInput={(e) => validate('zip', e.target.value)}
      />
      {isValid && <span className="check-mark">✓</span>}
    </div>
  );
}

// Child component: PaymentForm
export function PaymentForm() {
  const isValid = state.isValid;
  const data = state.data || {};

  const validate = (field: string, value: string) => {
    const newData = { ...data, [field]: value };
    const valid = newData.cardNumber && newData.cvv && newData.expiry;
    setState('isValid', valid);
    setState('data', newData);
  };

  return (
    <div className={`form-section ${isValid ? 'valid' : 'invalid'}`}>
      <h2>Payment</h2>
      <input
        id="card-input"
        type="text"
        placeholder="Card Number"
        value={data.cardNumber || ''}
        onInput={(e) => validate('cardNumber', e.target.value)}
      />
      <input
        id="cvv-input"
        type="text"
        placeholder="CVV"
        value={data.cvv || ''}
        onInput={(e) => validate('cvv', e.target.value)}
      />
      <input
        id="expiry-input"
        type="text"
        placeholder="Expiry (MM/YY)"
        value={data.expiry || ''}
        onInput={(e) => validate('expiry', e.target.value)}
      />
      {isValid && <span className="check-mark">✓</span>}
    </div>
  );
}

// Parent component: RegistrationPage
export default function RegistrationPage() {
  // Observe validation state from all form sections
  const personalValid = state["PersonalInfoForm.isValid"];
  const addressValid = state["AddressForm.isValid"];
  const paymentValid = state["PaymentForm.isValid"];

  const allValid = personalValid && addressValid && paymentValid;

  const invalidSections = [
    !personalValid && "Personal Info",
    !addressValid && "Address",
    !paymentValid && "Payment"
  ].filter(Boolean);

  const handleSubmit = () => {
    if (allValid) {
      // Submit registration
      const data = {
        personal: state["PersonalInfoForm.data"],
        address: state["AddressForm.data"],
        payment: state["PaymentForm.data"]
      };
      console.log('Submitting registration:', data);
      alert('Registration submitted successfully!');
    }
  };

  return (
    <div id="registration-root">
      <h1>Registration</h1>

      {/* Validation summary */}
      {!allValid && (
        <div id="validation-summary" className="validation-summary error">
          <strong>Please complete the following sections:</strong>
          <ul>
            {invalidSections.map(section => (
              <li key={section}>{section}</li>
            ))}
          </ul>
        </div>
      )}

      <Component name="PersonalInfoForm" state={{ isValid: false, data: {} }}>
        <PersonalInfoForm />
      </Component>

      <Component name="AddressForm" state={{ isValid: false, data: {} }}>
        <AddressForm />
      </Component>

      <Component name="PaymentForm" state={{ isValid: false, data: {} }}>
        <PaymentForm />
      </Component>

      {/* Submit button disabled until all valid */}
      <button
        id="submit-btn"
        type="button"
        onClick={handleSubmit}
        disabled={!allValid}
        className={allValid ? "btn-primary" : "btn-disabled"}
      >
        Complete Registration
      </button>

      {/* Status display for testing */}
      <div id="status" className="status">
        <p>Personal Valid: <span id="personal-valid">{personalValid ? 'Yes' : 'No'}</span></p>
        <p>Address Valid: <span id="address-valid">{addressValid ? 'Yes' : 'No'}</span></p>
        <p>Payment Valid: <span id="payment-valid">{paymentValid ? 'Yes' : 'No'}</span></p>
        <p>All Valid: <span id="all-valid">{allValid ? 'Yes' : 'No'}</span></p>
      </div>
    </div>
  );
}
